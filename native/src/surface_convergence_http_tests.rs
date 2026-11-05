//! 真实 TLS 的 POST → 成功观察 → 独立快照刷新；预算/取消不重放动作。
use super::*;
use crate::{live_relay_http::ConvergenceError, surface_action_ack::Ack};
use std::time::{Duration, Instant};
#[path = "surface_confirmation_http_tests.rs"]
mod confirmations;
fn ack(event: &ClickEvent, status: &str) -> Ack {
    Ack::parse(
        &serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.surface.v1",
        "instanceId":event.binding().instance(),"eventId":event.id(),"requestId":"request:1",
        "accepted":true,"status":status}))
        .unwrap(),
        event,
    )
    .unwrap()
}
fn snapshot(revision: u64, text: &str) -> serde_json::Value {
    serde_json::json!({"method":"loom.surface.snapshot","params":{"generation":2,"snapshot":{
        "protocolVersion":"loom.surface.v1","instanceId":"surface:test/part","attachmentId":"attachment:1",
        "revision":revision,"scene":{"id":"root","type":"text","props":{"text":text}}
    }}})
}
#[test]
fn actual_submission_converges_only_after_separate_success_refresh() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let mut event = event(&owner);
    let expected_id = event.id().to_owned();
    let session = session(&client.origin, u64::MAX);
    let peer = std::thread::spawn(move || {
        for index in 0..3 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, _) = request(&mut stream);
            let ack = serde_json::json!({"protocolVersion":"loom.surface.v1","instanceId":"surface:test/part",
                "eventId":expected_id,"requestId":"request:1","accepted":true,"status":if index == 0 { "running" } else { "succeeded" }});
            let response = if index == 0 {
                assert!(head.starts_with("POST /v1/surfaces/instances/surface:test%2Fpart/events "));
                ack
            } else {
                assert!(head.starts_with("GET /v1/surfaces/stream?after=0&timeoutMs=0 "));
                let messages = if index == 1 {
                    vec![
                        snapshot(3, "old"),
                        serde_json::json!({"method":"loom.surface.action.ack","params":ack}),
                    ]
                } else {
                    vec![snapshot(4, "fresh")]
                };
                serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":index+1,"reset":true,"messages":messages})
            };
            reply(&mut stream, 200, &serde_json::to_vec(&response).unwrap());
        }
    });
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    let initial = control.submit_surface_click(&owner, 3, &mut event).unwrap();
    let result =
        control.converge_surface(&event, &initial, Instant::now() + Duration::from_secs(5));
    assert!(result.ack.succeeded());
    let snapshot = result.result.unwrap();
    assert_eq!(snapshot.revision, 4);
    assert_eq!(snapshot.scene.root().props()["text"], "fresh");
    assert!(event.attempted());
    peer.join().unwrap();
}
#[test]
fn expired_cancelled_and_confirmation_states_do_not_issue_stream_requests() {
    let (_listener, client, _config) = fixture();
    let owner = active(&client.origin);
    let event = event(&owner);
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    assert!(matches!(
        control
            .converge_surface(&event, &ack(&event, "running"), Instant::now())
            .result,
        Err(ConvergenceError::Deadline)
    ));
    assert!(matches!(
        control
            .converge_surface(
                &event,
                &ack(&event, "awaiting_confirmation"),
                Instant::now() + Duration::from_secs(1)
            )
            .result,
        Err(ConvergenceError::ConfirmationRequired)
    ));
    cancel.cancel();
    assert!(matches!(
        control
            .converge_surface(
                &event,
                &ack(&event, "running"),
                Instant::now() + Duration::from_secs(1)
            )
            .result,
        Err(ConvergenceError::Stopped)
    ));
}
#[test]
fn cancellation_stops_active_convergence_and_keeps_last_ack() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let event = event(&owner);
    let initial = ack(&event, "running");
    let session = session(&client.origin, u64::MAX);
    let cancel = std::sync::Arc::new(Cancel::new());
    let task = cancel.clone();
    let (tx, rx) = std::sync::mpsc::channel();
    let peer = std::thread::spawn(move || {
        let mut stream =
            StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
        request(&mut stream);
        reply(
            &mut stream,
            200,
            br#"{"protocolVersion":"loom.surface-stream.v1","next":1,"reset":false,"messages":[]}"#,
        );
        tx.send(()).unwrap();
    });
    let worker = std::thread::spawn(move || {
        Control::new(&client, &session, &task).converge_surface(
            &event,
            &initial,
            Instant::now() + Duration::from_secs(5),
        )
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let start = Instant::now();
    cancel.cancel();
    let outcome = worker.join().unwrap();
    assert!(matches!(outcome.result, Err(ConvergenceError::Stopped)));
    assert_eq!(
        outcome.ack.status(),
        crate::surface_action_ack::Status::Running
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    peer.join().unwrap();
}
