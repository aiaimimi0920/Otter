//! 真实 HTTPS 目录后台查询、在途取消和迟到成功抑制。
use super::*;
use crate::{
    device_authorization_lease::Grant,
    live_relay_catalog_runtime::{Error as JobError, Job},
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
#[test]
fn successful_catalog_is_origin_bound_and_late_success_is_discarded_after_revoke() {
    for revoked in [false, true] {
        let (listener, client, config) = fixture();
        let origin = client.origin.clone();
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let lease = grant.lease(&origin, 0).unwrap();
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/live/sessions HTTP/1.1"));
            assert!(head
                .to_ascii_lowercase()
                .contains("authorization: device fixture-secret"));
            let entry: serde_json::Value = serde_json::from_slice(&snapshot()).unwrap();
            reply(
                &mut stream,
                200,
                &serde_json::to_vec(
                    &serde_json::json!({"protocolVersion":"loom.live.v1","sessions":[entry]}),
                )
                .unwrap(),
            );
        });
        let mut job = Job::start_prepared(client, lease).unwrap();
        wait(&job);
        if revoked {
            drop(grant);
        }
        let result = job.try_join().unwrap();
        if revoked {
            assert!(matches!(result, Err(JobError::Stopped)));
        } else {
            let catalog = result.unwrap();
            assert_eq!(catalog.origin, origin);
            assert_eq!(catalog.entries.len(), 1);
        }
        assert!(job.wait_handle().is_none());
        assert!(job.try_join().is_none());
        peer.join().unwrap();
    }
}
#[test]
fn cancellation_interrupts_inflight_catalog_request() {
    let (listener, client, config) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let (sent_tx, sent_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        sent_tx.send(()).unwrap();
        let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
    });
    let mut job = Job::start_prepared(client, lease).unwrap();
    sent_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let start = std::time::Instant::now();
    job.request_stop();
    wait(&job);
    let elapsed = start.elapsed();
    let result = job.try_join().unwrap();
    let _ = finish_tx.send(());
    peer.join().unwrap();
    assert!(matches!(result, Err(JobError::Stopped)));
    assert!(elapsed < std::time::Duration::from_secs(2), "{elapsed:?}");
}

#[test]
fn completed_catalog_and_viewer_tasks_share_capacity_until_reaped() {
    use crate::live_relay_control_runtime::{Exit, Runtime, Target};
    let (_listener, client, _config) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let mut leases: Vec<_> = (0..6)
        .map(|_| grant.lease(&client.origin, 0).unwrap())
        .collect();
    drop(grant);
    let mut jobs: Vec<_> = (0..3)
        .map(|_| Job::start_prepared(client.clone(), leases.pop().unwrap()).unwrap())
        .collect();
    let mut viewer = Runtime::start(
        client.clone(),
        leases.pop().unwrap(),
        Target {
            session: "live:test/part".into(),
            surface: "surface-1".into(),
            attachment: "attachment-1".into(),
            expected: None,
            selected: None,
        },
        |_| panic!("revoked task must not consume events"),
    )
    .unwrap();
    for job in &jobs {
        wait(job);
    }
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(viewer.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
    assert!(matches!(
        Job::start_prepared(client.clone(), leases.pop().unwrap()),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
    ));
    assert_eq!(viewer.try_join(), Some(Exit::Stopped));
    let mut replacement = Job::start_prepared(client, leases.pop().unwrap()).unwrap();
    wait(&replacement);
    assert!(matches!(
        replacement.try_join(),
        Some(Err(JobError::Stopped))
    ));
    for job in &mut jobs {
        assert!(matches!(job.try_join(), Some(Err(JobError::Stopped))));
    }
}
