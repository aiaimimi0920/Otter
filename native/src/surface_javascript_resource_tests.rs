//! 真 HTTPS/Device/资源租约→真实 JS child；不声明 Application/FIFO/真实 Loom 已接通。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    surface_javascript::authorized::{Error as JsError, Job},
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
#[path = "surface_javascript_authorization_tests.rs"]
mod authorization;
#[path = "surface_javascript_owner_tests.rs"]
mod runtime;
fn script(origin: &str, bytes: &[u8]) -> MountedSurface {
    let (mut mount, id) = image(origin, bytes);
    mutate(&mut mount, |value| {
        value["runtime"] = "javascript".into();
        value["entryResourceId"] = id.into();
        value["resourceLeases"][0]["resource"]["kind"] = "binary".into();
        value["resourceLeases"][0]["resource"]["mime"] = "application/javascript".into();
        value["scene"] = crate::surface_javascript::fixture::snapshot(3, 42)["scene"].clone();
        value["state"] = serde_json::json!({"quote":42});
    });
    mount
}
fn activate(mount: &mut MountedSurface) {
    let mut value = mounted();
    value["instance"]["attachments"]["attachment:1"]["lifecycle"] = "active".into();
    value["instance"]["attachments"]["attachment:1"]["lifecycleRevision"] = 8.into();
    mount.binding = Binding::from_mount(
        &serde_json::to_vec(&value).unwrap(),
        mount.binding.origin(),
        "device-1",
        "node:1",
    )
    .unwrap();
}
fn wait(job: &Job) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 8000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn javascript_entry_rejects_wrong_runtime_mime_transport_size_and_duplicate_before_network() {
    for (pointer, value) in [
        ("/runtime", serde_json::json!("declarative")),
        ("/entryResourceId", serde_json::json!("sha256:foreign")),
        (
            "/resourceLeases/0/resource/kind",
            serde_json::json!("image"),
        ),
        (
            "/resourceLeases/0/resource/mime",
            serde_json::json!("text/html"),
        ),
        (
            "/resourceLeases/0/resource/size",
            serde_json::json!(1024 * 1024 + 1),
        ),
        (
            "/resourceLeases/0/transport/kind",
            serde_json::json!("shared_memory"),
        ),
        (
            "/resourceLeases/0/transport/path",
            serde_json::json!("https://foreign.invalid/code.js"),
        ),
        ("/resourceLeases/0/expiresAtMs", serde_json::json!(0)),
        ("/attachmentId", serde_json::json!("foreign")),
    ] {
        let mut mount = script("https://localhost:1234", b"source");
        mutate(&mut mount, |s| *s.pointer_mut(pointer).unwrap() = value);
        assert!(mount.javascript_entry(clock()).is_err(), "{pointer}");
    }
    let mut mount = script("https://localhost:1234", b"source");
    assert!(mount
        .image_resource(
            serde_json::from_str::<serde_json::Value>(mount.snapshot.get()).unwrap()
                ["entryResourceId"]
                .as_str()
                .unwrap(),
            clock()
        )
        .is_err());
    mutate(&mut mount, |s| {
        let duplicate = s["resourceLeases"][0].clone();
        s["resourceLeases"].as_array_mut().unwrap().push(duplicate);
    });
    assert!(matches!(
        mount.javascript_entry(clock()),
        Err(ResourceError::Identity)
    ));
}
#[test]
fn javascript_authenticated_source_projects_to_hwnd_rebinds_updates_and_reaps() {
    let _serial = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    let _apartment = crate::desktop::surface_window::SurfaceWindow::javascript_apartment_for_test();
    let (listener, client, config) = fixture();
    let source = crate::surface_javascript::fixture::SOURCE.as_bytes();
    let mut mount = script(&client.origin, source);
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let destination = grant.lease(&client.origin, clock()).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        let head = head.to_ascii_lowercase();
        assert!(head.starts_with("get /v1/surfaces/resources/"));
        for header in [
            "authorization: device fixture-secret",
            "x-loom-device-nonce:",
            "x-loom-surface-lease: lease:fixture",
        ] {
            assert!(head.contains(header));
        }
        reply(&mut stream, 200, source);
    });
    let input = MountedSurface {
        binding: mount.binding.clone(),
        snapshot: mount.snapshot.clone(),
    };
    let mut job = Job::start_prepared(
        client.clone(),
        grant.lease(&client.origin, clock()).unwrap(),
        input,
    )
    .unwrap();
    wait(&job);
    let ready = job.try_join().unwrap().unwrap();
    peer.join().unwrap();
    let mut runtime = ready.transfer(&destination, &mount).unwrap();
    drop(job); // 成功 join 的任务释放不能撤销已移交的运行时。
    let probe = runtime.probe();
    let frame = runtime.take_initial(&mount).unwrap();
    let target = frame.scene.root().children()[1].id().to_owned();
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        "原生🙂:42:0"
    );
    let window = crate::desktop::surface_window::SurfaceWindow::new(frame.scene, 320, 200).unwrap();
    let node = window.accessibility_node("dom:2").unwrap();
    activate(&mut mount);
    let target = window.javascript_paint_click_for_test(&target, &mount.binding, 3);
    let frame = runtime.click(&mount, &target).unwrap();
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        "原生🙂:42:1"
    );
    assert_eq!(frame.events[0].binding(), &mount.binding);
    window.replace(frame.scene).unwrap();
    mutate(&mut mount, |s| {
        s["revision"] = 4.into();
        s["state"]["quote"] = 45.into();
        s["resourceLeases"][0]["leaseId"] = "renewed:entry".into();
    });
    let frame = runtime.update(&mount).unwrap();
    assert_eq!(
        crate::surface_javascript::fixture::text(&frame.scene),
        "原生🙂:45:1"
    );
    window.replace(frame.scene).unwrap();
    runtime.suspend(&mount).unwrap();
    runtime.resume(&mount).unwrap();
    let frame = runtime.click(&mount, &target).unwrap();
    assert_eq!(frame.events[0].revision(), 4);
    runtime.dispose(&mount).unwrap();
    assert!(probe.exited());
    window.close().unwrap();
    assert!(!window.is_alive() && node.read().is_none());
}
#[test]
fn javascript_resource_corruption_invalid_utf8_redirect_and_401_fail_closed() {
    for (status, bytes) in [
        (200, b"evil".as_slice()),
        (200, b"\xff\xfe\xff\xfe".as_slice()),
        (302, b"".as_slice()),
        (401, b"".as_slice()),
    ] {
        let (listener, client, config) = fixture();
        let expected = if bytes.first() == Some(&255) {
            bytes
        } else {
            b"data"
        };
        let mount = script(&client.origin, expected);
        let mut grant = Grant::new(session(&client.origin, u64::MAX));
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            request(&mut stream);
            reply(&mut stream, status, bytes);
        });
        let mut job = Job::start_prepared(
            client.clone(),
            grant.lease(&client.origin, clock()).unwrap(),
            mount,
        )
        .unwrap();
        wait(&job);
        assert!(job.try_join().unwrap().is_err(), "{status}");
        peer.join().unwrap();
    }
}
