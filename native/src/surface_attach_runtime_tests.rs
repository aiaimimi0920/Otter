use super::*;
#[path = "surface_form_attach_tests.rs"]
mod forms;
#[path = "surface_recovery_runtime_tests.rs"]
mod recovery;
use crate::{
    device_authorization_lease::Grant,
    live_relay_http::Persistence,
    surface_attach_runtime::{Error as JobError, Job, Request as AttachRequest},
    surface_host_capabilities::{Input, Report, Runtime},
};
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
fn request_data() -> Arc<AttachRequest> {
    Arc::new(
        AttachRequest::new(
            "art:1".into(),
            "node:1".into(),
            Report::new(
                vec![Runtime::Declarative],
                vec!["text".into()],
                vec![],
                vec![],
                Input::default(),
            )
            .unwrap(),
            Persistence::Persistent,
        )
        .unwrap(),
    )
}
fn wait(job: &Job) {
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(job.wait_handle().unwrap().as_raw_handle()), 5000) },
        WAIT_OBJECT_0
    );
}
#[test]
fn candidate_recovery_uses_background_mount_and_retains_evidence_after_cancel() {
    for cancel_in_flight in [true, false] {
        let (listener, client, config) = fixture();
        let origin = client.origin.clone();
        let query_session = session(&origin, u64::MAX);
        let mut grant = Grant::new(session(&origin, u64::MAX));
        let lease = grant.lease(&origin, 0).unwrap();
        let (sent_tx, sent_rx) = std::sync::mpsc::channel();
        let (finish_tx, finish_rx) = std::sync::mpsc::channel();
        let peer = std::thread::spawn(move || {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/surfaces/stream?"));
            let response = serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":1,"reset":false,
                "messages":[{"method":"loom.surface.snapshot","params":{"hookNodeId":"node:1","generation":2,
                    "snapshot":{"protocolVersion":"loom.surface.v1","instanceId":"surface:test/part","attachmentId":"attachment:1","artId":"art:1","revision":3}}}]});
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            drop(stream);
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/surfaces/instances/surface:test%2Fpart/mount "));
            assert_eq!(body, serde_json::json!({"attachmentId":"attachment:1"}));
            sent_tx.send(()).unwrap();
            if cancel_in_flight {
                let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
            } else {
                let mut response = mounted();
                response["instance"]["descriptor"]["artId"] = "art:1".into();
                response["instance"]["attachments"]["attachment:1"]["snapshot"]["artId"] =
                    "art:1".into();
                reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
            }
        });
        let cancel = Cancel::new();
        let candidate = Control::new(&client, &query_session, &cancel)
            .surface_recovery_candidate("art:1", "node:1")
            .unwrap()
            .unwrap();
        let data = Arc::new(AttachRequest::recover(
            Arc::new(candidate),
            Report::new(
                vec![Runtime::Declarative],
                vec!["text".into()],
                vec![],
                vec![],
                Input::default(),
            )
            .unwrap(),
        ));
        let mut job = Job::start_prepared(client, lease, data).unwrap();
        sent_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        let start = std::time::Instant::now();
        if cancel_in_flight {
            job.request_stop();
        }
        wait(&job);
        drop(grant);
        let outcome = job.try_join().unwrap();
        let _ = finish_tx.send(());
        peer.join().unwrap();
        assert!(matches!(outcome.result, Err(JobError::Stopped)));
        assert_eq!(outcome.uncertain, cancel_in_flight);
        assert_eq!(outcome.binding.is_some(), !cancel_in_flight);
        assert_eq!(
            outcome.request.recovery_candidate().unwrap().instance(),
            "surface:test/part"
        );
        if cancel_in_flight {
            assert!(start.elapsed() < std::time::Duration::from_secs(2));
        }
        assert!(job.wait_handle().is_none());
    }
}
#[test]
fn cancelling_sent_attach_preserves_unknown_creation_and_request_context() {
    let (listener, client, config) = fixture();
    let origin = client.origin.clone();
    let mut grant = Grant::new(session(&origin, u64::MAX));
    let lease = grant.lease(&origin, 0).unwrap();
    let (sent_tx, sent_rx) = std::sync::mpsc::channel();
    let (finish_tx, finish_rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        sent_tx.send(()).unwrap();
        let _ = finish_rx.recv_timeout(std::time::Duration::from_secs(8));
    });
    let mut job = Job::start_prepared(client, lease, request_data()).unwrap();
    sent_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    let start = std::time::Instant::now();
    job.request_stop();
    wait(&job);
    let outcome = job.try_join().unwrap();
    let elapsed = start.elapsed();
    let _ = finish_tx.send(());
    peer.join().unwrap();
    assert!(outcome.uncertain && outcome.binding.is_none());
    assert!(matches!(outcome.result, Err(JobError::Stopped)));
    assert_eq!(outcome.origin, origin);
    assert_eq!(outcome.device, "device-1");
    assert_eq!(outcome.request.art(), "art:1");
    assert_eq!(outcome.request.node(), "node:1");
    assert!(elapsed < std::time::Duration::from_secs(2));
    assert!(job.try_join().is_none() && job.wait_handle().is_none());
}
#[test]
fn revoked_completed_attach_keeps_confirmed_binding_without_scene_delivery() {
    let (listener, client, config) = fixture();
    let mut grant = Grant::new(session(&client.origin, u64::MAX));
    let lease = grant.lease(&client.origin, 0).unwrap();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        let mut response = mounted();
        response["instance"]["descriptor"]["artId"] = "art:1".into();
        response["instance"]["attachments"]["attachment:1"]["snapshot"]["artId"] = "art:1".into();
        reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
    });
    let mut job = Job::start_prepared(client, lease, request_data()).unwrap();
    wait(&job);
    drop(grant);
    let outcome = job.try_join().unwrap();
    assert!(!outcome.uncertain);
    assert_eq!(outcome.binding.unwrap().attachment(), "attachment:1");
    assert!(matches!(outcome.result, Err(JobError::Stopped)));
    peer.join().unwrap();
}
