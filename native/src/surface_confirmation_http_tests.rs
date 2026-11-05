use super::*;
#[path = "surface_confirmation_decision_http_tests.rs"]
mod decisions;
#[test]
fn device_stream_returns_validated_confirmation_without_posting_decision() {
    let (listener, client, config) = fixture();
    let owner = active(&client.origin);
    let mut event = event(&owner);
    event.begin_submission();
    let initial = ack(&event, "awaiting_confirmation");
    let action: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    let confirmation = serde_json::json!({"protocolVersion":"loom.surface.v1","confirmationId":"confirmation:test",
        "instanceId":event.binding().instance(),"attachmentId":event.binding().attachment(),
        "deviceId":event.binding().device(),"hookNodeId":event.binding().node(),
        "eventId":event.id(),"requestId":initial.request_id(),"actionId":action["action"],"risk":"high","expiresAtMs":u64::MAX});
    let peer =
        std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(config).unwrap(), accept(&listener));
            let (head, _) = request(&mut stream);
            assert!(head.starts_with("GET /v1/surfaces/stream?after=0&timeoutMs=0 "));
            reply(&mut stream, 200, &serde_json::to_vec(&serde_json::json!({
            "protocolVersion":"loom.surface-stream.v1","next":1,"reset":true,
            "messages":[{"method":"loom.surface.confirmation.request","params":confirmation}]
        })).unwrap());
        });
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let outcome = Control::new(&client, &session, &cancel).surface_confirmation(&event, &initial);
    assert_eq!(outcome.result.unwrap().id(), "confirmation:test");
    assert_eq!(
        outcome.ack.status(),
        crate::surface_action_ack::Status::AwaitingConfirmation
    );
    peer.join().unwrap();
}
#[test]
fn unsubmitted_completed_and_cancelled_confirmation_fetches_do_not_connect() {
    let (_listener, client, _config) = fixture();
    let mut event = event(&active(&client.origin));
    let session = session(&client.origin, u64::MAX);
    let cancel = Cancel::new();
    let control = Control::new(&client, &session, &cancel);
    assert!(matches!(
        control
            .surface_confirmation(&event, &ack(&event, "awaiting_confirmation"))
            .result,
        Err(crate::live_relay_http::ConfirmationError::State)
    ));
    event.begin_submission();
    assert!(matches!(
        control
            .surface_confirmation(&event, &ack(&event, "succeeded"))
            .result,
        Err(crate::live_relay_http::ConfirmationError::State)
    ));
    cancel.cancel();
    assert!(matches!(
        control
            .surface_confirmation(&event, &ack(&event, "awaiting_confirmation"))
            .result,
        Err(crate::live_relay_http::ConfirmationError::Stopped)
    ));
}
