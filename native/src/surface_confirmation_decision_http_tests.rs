use super::*;
#[path = "surface_decision_runtime_tests.rs"]
mod runtime;
use crate::{
    live_relay_http::DecisionSubmitError,
    surface_confirmation::{Choice, Decision, Request as Prompt},
};
fn decision(event: &ClickEvent, ack: &Ack, choice: Choice, expires: u64) -> Decision {
    let action: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    let raw = serde_json::value::RawValue::from_string(serde_json::json!({
        "protocolVersion":"loom.surface.v1","confirmationId":"confirmation:test",
        "instanceId":event.binding().instance(),"attachmentId":event.binding().attachment(),
        "deviceId":event.binding().device(),"hookNodeId":event.binding().node(),
        "eventId":event.id(),"requestId":ack.request_id(),"actionId":action["action"],"risk":"high","expiresAtMs":expires
    }).to_string()).unwrap();
    let prompt = Prompt::parse_matching(&raw, event, ack).unwrap().unwrap();
    Decision::new(prompt, event, ack, choice, 0).unwrap_or_else(|_| panic!("prepare"))
}
#[test]
fn approval_posts_once_then_observes_original_action_and_independent_snapshot() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let mut event = event(&owner);
    event.begin_submission();
    let initial = ack(&event, "awaiting_confirmation");
    let mut decision = decision(&event, &initial, Choice::Approve, u64::MAX);
    let id = event.id().to_owned();
    let peer = std::thread::spawn(move || {
        for index in 0..3 {
            let mut stream = StreamOwned::new(
                ServerConnection::new(config.clone()).unwrap(),
                accept(&listener),
            );
            let (head, body) = request(&mut stream);
            if index == 0 {
                assert!(head.starts_with("POST /v1/surfaces/confirmations/decision "));
                assert_eq!(body["approved"], true);
                assert_eq!(body["deviceId"], "device-1");
                reply(&mut stream, 204, b"");
            } else {
                assert!(head.starts_with("GET /v1/surfaces/stream?after=0&timeoutMs=0 "));
                let messages = if index == 1 {
                    vec![
                        serde_json::json!({"method":"loom.surface.action.ack","params":{
                    "protocolVersion":"loom.surface.v1","instanceId":"surface:test/part","eventId":id,
                    "requestId":"request:1","accepted":true,"status":"succeeded"}}),
                    ]
                } else {
                    vec![snapshot(4, "fresh")]
                };
                reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({"protocolVersion":"loom.surface-stream.v1","next":index,"reset":true,"messages":messages})).unwrap());
            }
        }
    });
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    control
        .submit_surface_confirmation(&owner, 3, &event, &initial, &mut decision)
        .unwrap();
    assert!(decision.attempted() && decision.confirmed());
    assert_eq!(
        control.submit_surface_confirmation(&owner, 3, &event, &initial, &mut decision),
        Err(DecisionSubmitError::AlreadyAttempted)
    );
    let outcome = control.converge_decided_surface(
        &event,
        &initial,
        &decision,
        Instant::now() + Duration::from_secs(5),
    );
    assert!(outcome.ack.succeeded());
    assert_eq!(outcome.result.unwrap().revision, 4);
    peer.join().unwrap();
}
#[test]
fn rejection_and_transport_failure_never_replay_or_claim_action_success() {
    for status in [204, 401] {
        let (listener, client, config) = fixture();
        let owner = active(&client.origin);
        let mut event = event(&owner);
        event.begin_submission();
        let initial = ack(&event, "awaiting_confirmation");
        let mut decision = decision(&event, &initial, Choice::Reject, u64::MAX);
        let peer = std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/surfaces/confirmations/decision "));
            assert_eq!(body["approved"], false);
            reply(&mut stream, status, b"");
        });
        let session = session(&client.origin, u64::MAX);
        let cancel = Cancel::new();
        let control = Control::new(&client, &session, &cancel);
        assert_eq!(
            control
                .submit_surface_confirmation(&owner, 3, &event, &initial, &mut decision)
                .is_ok(),
            status == 204
        );
        assert!(decision.attempted());
        assert_eq!(decision.confirmed(), status == 204);
        assert!(!initial.succeeded());
        assert_eq!(
            control.submit_surface_confirmation(&owner, 3, &event, &initial, &mut decision),
            Err(DecisionSubmitError::AlreadyAttempted)
        );
        if status == 401 {
            assert!(matches!(
                control
                    .converge_decided_surface(&event, &initial, &decision, Instant::now())
                    .result,
                Err(ConvergenceError::Identity)
            ));
        }
        peer.join().unwrap();
    }
}
#[test]
fn stale_expired_and_cancelled_decisions_do_not_attempt_network() {
    let (_listener, client, _config) = fixture();
    let owner = active(&client.origin);
    let mut event = event(&owner);
    event.begin_submission();
    let initial = ack(&event, "awaiting_confirmation");
    let mut pending = decision(&event, &initial, Choice::Approve, 1);
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    assert_eq!(
        control.submit_surface_confirmation(&owner, 4, &event, &initial, &mut pending),
        Err(DecisionSubmitError::Stale)
    );
    assert!(matches!(
        control.submit_surface_confirmation(&owner, 3, &event, &initial, &mut pending),
        Err(DecisionSubmitError::Request(
            crate::surface_confirmation::Error::Expired
        ))
    ));
    cancel.cancel();
    assert_eq!(
        control.submit_surface_confirmation(&owner, 3, &event, &initial, &mut pending),
        Err(DecisionSubmitError::Stopped)
    );
    assert!(!pending.attempted());
}
