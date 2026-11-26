//! Real socket cancellation, authorization revocation, and unconsumed-result capacity in fetch threads; no production Loom access.
use super::*;
#[path = "surface_image_inline_tests.rs"]
mod inline;
use crate::{
    device_authorization_lease::Grant,
    surface_resource_runtime::{Error as JobError, Job, Ready},
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
fn wait(job: &Job) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
fn image(origin: &str, bytes: &[u8]) -> (MountedSurface, String) {
    let (mut mount, id) = super::image(origin, bytes);
    mutate(&mut mount, |s| {
        let r = &mut s["resourceLeases"][0]["resource"];
        r["mime"] = "application/x-neuro-rgba8".into();
        r["width"] = 1.into();
        r["height"] = 1.into();
    });
    (mount, id)
}
fn complete() -> (Grant, Ready, MountedSurface) {
    let (listener, client, config) = fixture();
    let (mount, id) = image(&client.origin, b"data");
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, clock()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, b"data");
    });
    let mut job =
        Job::start_prepared(client, lease, mount.image_resource(&id, clock()).unwrap()).unwrap();
    wait(&job);
    let ready = job.try_join().unwrap().unwrap();
    assert!(job.wait_handle().is_none() && job.try_join().is_none());
    drop(job);
    peer.join().unwrap();
    assert_eq!(
        &*ready.image(&mount, clock()).unwrap().pixels,
        &[44, 37, 38, 97]
    );
    (grant, ready, mount)
}

#[test]
fn configured_image_resource_rechecks_the_same_ca_path_for_every_job() {
    use crate::device_authorization_config::{Config, Error as PrepareError};
    let (listener, client, tls, pem) = fixture_with_host("127.0.0.1");
    let root = crate::test_temp_root().join(format!("otter-surface-ca-{:?}", unsafe {
        windows::Win32::System::Com::CoCreateGuid().unwrap()
    }));
    std::fs::create_dir(&root).unwrap();
    let ca = root.join("ca.pem");
    std::fs::write(&ca, pem).unwrap();
    let (mount, id) = image(&client.origin, b"data");
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let mut run = || {
        let config = Config::from_lookup(&client.origin, |key| match key {
            "OTTER_NATIVE_PROFILE_DIR" => Some(root.join("profile").into_os_string()),
            "OTTER_LOOM_TLS_ORIGIN" => Some(client.origin.clone().into()),
            "OTTER_LOOM_TLS_CA_FILE" => Some(ca.clone().into_os_string()),
            _ => None,
        })
        .unwrap();
        let lease = grant.lease(&client.origin, clock()).unwrap();
        let mut job =
            Job::start(config, lease, mount.image_resource(&id, clock()).unwrap()).unwrap();
        wait(&job);
        job.try_join().unwrap()
    };
    let peer = std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, b"data");
    });
    assert!(run().is_ok());
    peer.join().unwrap();
    std::fs::remove_file(&ca).unwrap();
    assert!(matches!(
        run(),
        Err(JobError::Preparation(PrepareError::Trust))
    ));
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn image_resource_transfer_releases_task_slot_but_keeps_destination_grant_gate() {
    let (mut grant, ready, mount) = complete();
    let destination = grant.lease(mount.binding.origin(), clock()).unwrap();
    let (request, pixels) = ready.transfer(&destination, &mount, clock()).unwrap();
    assert_eq!(&*pixels.pixels, &[44, 37, 38, 97]);
    let slots: Vec<_> = (0..4)
        .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
        .collect();
    assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
    destination
        .validate_resource(&request, &mount, clock())
        .unwrap();
    drop(grant);
    assert!(destination
        .validate_resource(&request, &mount, clock())
        .is_err());
    drop(slots);
}
#[test]
fn image_resource_transfer_rejects_equal_credentials_from_another_grant() {
    let (_grant, ready, mount) = complete();
    let mut other = Grant::new(session(mount.binding.origin(), u64::MAX));
    let destination = other.lease(mount.binding.origin(), clock()).unwrap();
    assert!(matches!(
        ready.transfer(&destination, &mount, clock()),
        Err(JobError::Resource(ResourceError::Identity))
    ));
}
#[test]
fn image_resource_job_ready_keeps_original_lease_and_scene_gate() {
    let (grant, ready, mut mount) = complete();
    assert_eq!(ready.mime(), "application/x-neuro-rgba8");
    assert_eq!(ready.revision(), 3);
    mutate(&mut mount, |s| s["revision"] = 4.into());
    assert!(matches!(
        ready.image(&mount, clock()),
        Err(JobError::Resource(ResourceError::Identity))
    ));
    mutate(&mut mount, |s| s["revision"] = 3.into());
    assert_eq!(
        &*ready.image(&mount, clock()).unwrap().pixels,
        &[44, 37, 38, 97]
    );
    // Passing a new Cancel must not bypass credential-cache revocation; the original Grant can still revoke Ready after Job cleanup.
    drop(grant);
    assert!(matches!(
        ready.image(&mount, clock()),
        Err(JobError::Stopped)
    ));
}
#[test]
fn image_resource_job_cancel_during_body_read_interrupts_socket_and_drains() {
    for drop_running in [false, true] {
        let (listener, client, config) = fixture();
        let (mount, id) = image(&client.origin, b"data");
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, clock()).unwrap();
        let (sent, received) = std::sync::mpsc::channel();
        let (finish, done) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\nd")
                .unwrap();
            stream.flush().unwrap();
            sent.send(()).unwrap();
            let _ = done.recv_timeout(std::time::Duration::from_secs(8));
        });
        let mut job =
            Job::start_prepared(client, lease, mount.image_resource(&id, clock()).unwrap())
                .unwrap();
        received
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let start = std::time::Instant::now();
        if drop_running {
            drop(job);
        } else {
            job.request_stop();
            wait(&job);
            assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        finish.send(()).unwrap();
        peer.join().unwrap();
    }
}
#[test]
fn image_resource_job_finished_before_revocation_never_delivers_success() {
    let (listener, client, config) = fixture();
    let (mount, id) = image(&client.origin, b"data");
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, clock()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, b"data");
    });
    let mut job =
        Job::start_prepared(client, lease, mount.image_resource(&id, clock()).unwrap()).unwrap();
    wait(&job);
    drop(grant);
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
    peer.join().unwrap();
}
#[test]
fn image_resource_job_unconsumed_ready_retains_shared_capacity() {
    let mut ready: Vec<_> = (0..4).map(|_| complete()).collect();
    let (_listener, client, _config) = fixture();
    let (mount, id) = image(&client.origin, b"data");
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, clock()).unwrap();
    assert!(
        matches!(Job::start_prepared(client, lease, mount.image_resource(&id, clock()).unwrap()),
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
    );
    ready.pop();
    let (_listener, client, _config) = fixture();
    let (mount, id) = image(&client.origin, b"data");
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, clock()).unwrap();
    lease.revoke();
    let mut job =
        Job::start_prepared(client, lease, mount.image_resource(&id, clock()).unwrap()).unwrap();
    wait(&job);
    assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
}
#[test]
fn image_resource_sessions_rejected_success_retained_then_cancelled_or_delivered() {
    use crate::desktop::composition::relay::sessions::Sessions;
    for mode in 0..3 {
        let (listener, client, config) = fixture();
        let (mount, id) = image(&client.origin, b"data");
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let lease = grant.lease(&client.origin, clock()).unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            reply(&mut stream, 200, b"data");
        });
        let mut sessions = Sessions::default();
        sessions
            .start_resource_prepared(client, lease, mount.image_resource(&id, clock()).unwrap())
            .unwrap();
        let handles: Vec<_> = sessions
            .wait_handles()
            .map(|h| HANDLE(h.as_raw_handle()))
            .collect();
        assert_eq!(handles.len(), 1);
        assert_eq!(
            unsafe { WaitForSingleObject(handles[0], 5000) },
            WAIT_OBJECT_0
        );
        assert!(sessions.poll_surfaces().is_empty());
        assert!(sessions.finished());
        assert!(sessions.resource_deadline(clock()).is_some());
        assert_eq!(sessions.wait_handles().count(), 0);
        let mut seen = false;
        assert!(!sessions.deliver_surface_resource(|ready| {
            seen = true;
            assert_eq!(
                &*ready.image(&mount, clock()).unwrap().pixels,
                &[44, 37, 38, 97]
            );
            Err(Box::new(ready))
        }));
        assert!(seen);
        if mode == 2 {
            sessions.expire_surface_resources(u64::MAX);
            assert!(sessions.resource_deadline(clock()).is_none());
            assert!(!sessions.deliver_surface_resource(|_| panic!("expired ready retained")));
        } else if mode == 1 {
            sessions.cancel_surface_resources(|binding, revision| {
                binding == &mount.binding && revision == 3
            });
            assert!(!sessions.deliver_surface_resource(|_| panic!("cancelled ready retained")));
        } else {
            assert!(sessions.deliver_surface_resource(|ready| {
                assert_eq!(
                    &*ready.image(&mount, clock()).unwrap().pixels,
                    &[44, 37, 38, 97]
                );
                Ok(())
            }));
        }
        assert!(!sessions.deliver_surface_resource(|_| panic!("delivered twice")));
        assert!(sessions.resource_deadline(clock()).is_none());
        // Both acceptance and cancellation release shared slots rather than leaving them occupied by dangling references from prior Job/Ready owners.
        let slots: Vec<_> = (0..4)
            .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
            .collect();
        drop(slots);
        peer.join().unwrap();
    }
}
#[test]
fn image_resource_worker_decodes_png_before_cross_thread_delivery() {
    let (listener, client, config) = fixture();
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255, 0, 0, 255]).unwrap();
        writer.finish().unwrap();
    }
    let (mount, id) = super::image(&client.origin, &encoded);
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, clock()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(&mut stream, 200, &encoded);
    });
    let mut job =
        Job::start_prepared(client, lease, mount.image_resource(&id, clock()).unwrap()).unwrap();
    wait(&job);
    let ready = job.try_join().unwrap().unwrap();
    let decoded = ready.image(&mount, clock()).unwrap();
    assert_eq!((decoded.width, decoded.height), (1, 1));
    assert_eq!(&*decoded.pixels, &[0, 0, 255, 255]);
    let weak = decoded.pixels.downgrade();
    drop(job);
    drop(ready);
    assert!(weak.upgrade().is_none());
    peer.join().unwrap();
}
