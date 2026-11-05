use super::*;
use crate::surface_confirmation::{Error as ConfirmationError, Request, Risk};
fn request(event: &ClickEvent) -> serde_json::Value {
    let action: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    serde_json::json!({"protocolVersion":"loom.surface.v1","confirmationId":"confirmation:test",
        "instanceId":event.binding().instance(),"attachmentId":event.binding().attachment(),
        "deviceId":event.binding().device(),"hookNodeId":event.binding().node(),
        "eventId":event.id(),"requestId":"request:test","actionId":action["action"],
        "risk":"high","expiresAtMs":1000,"payload":{"message":"untrusted"}})
}
fn raw(value: serde_json::Value) -> Box<serde_json::value::RawValue> {
    serde_json::value::RawValue::from_string(value.to_string()).unwrap()
}
#[test]
fn confirmation_requires_attempted_pending_action_and_rechecks_expiry() {
    let mut event = event();
    let ack = initial(&event, "awaiting_confirmation");
    let request = Request::parse_matching(&raw(request(&event)), &event, &ack)
        .unwrap()
        .unwrap();
    assert_eq!(request.risk(), Risk::High);
    assert_eq!(
        request.validate(&event, &ack, 999),
        Err(ConfirmationError::State)
    );
    event.begin_submission();
    assert!(request.validate(&event, &ack, 999).is_ok());
    assert_eq!(
        request.validate(&event, &ack, 1000),
        Err(ConfirmationError::Expired)
    );
    assert_eq!(
        request.validate(&event, &initial(&event, "succeeded"), 999),
        Err(ConfirmationError::State)
    );
    assert!(request.payload().unwrap().get().contains("untrusted"));
}
#[test]
fn confirmation_identity_risk_and_budget_fail_closed() {
    let event = event();
    let ack = initial(&event, "awaiting_confirmation");
    for field in [
        "attachmentId",
        "deviceId",
        "hookNodeId",
        "actionId",
        "risk",
        "protocolVersion",
    ] {
        let mut value = request(&event);
        value[field] = "other".into();
        assert!(
            Request::parse_matching(&raw(value), &event, &ack).is_err(),
            "{field}"
        );
    }
    for field in ["instanceId", "eventId", "requestId"] {
        let mut value = request(&event);
        value[field] = "other".into();
        assert!(Request::parse_matching(&raw(value), &event, &ack)
            .unwrap()
            .is_none());
    }
    let mut value = request(&event);
    value["payload"] = "x".repeat(crate::surface_confirmation::MAX_BYTES).into();
    assert!(matches!(
        Request::parse_matching(&raw(value), &event, &ack),
        Err(ConfirmationError::Limit)
    ));
}
#[test]
fn whole_batch_failure_or_changed_prompt_prevents_confirmation_delivery() {
    let event = event();
    let initial = initial(&event, "awaiting_confirmation");
    let message =
        serde_json::json!({"method":"loom.surface.confirmation.request","params":request(&event)});
    for tail in [message_ack(&event, "failed"), {
        let mut changed = message.clone();
        changed["params"]["expiresAtMs"] = 2000.into();
        changed
    }] {
        let mut state = Tracker::new(&event, &initial).unwrap();
        assert!(state
            .apply(&batch(0, 1, vec![message.clone(), tail]))
            .is_err());
        assert!(state.take_confirmation().is_none());
    }
    let mut state = Tracker::new(&event, &initial).unwrap();
    state.apply(&batch(0, 1, vec![message])).unwrap();
    assert_eq!(state.take_confirmation().unwrap().id(), "confirmation:test");
    assert!(state.take_confirmation().is_none());
}

#[test]
fn decision_requires_explicit_choice_and_failed_preparation_returns_request() {
    use crate::surface_confirmation::{Choice, Decision};
    let mut event = event();
    event.begin_submission();
    let ack = initial(&event, "awaiting_confirmation");
    for choice in [Choice::Approve, Choice::Reject] {
        let request = Request::parse_matching(&raw(request(&event)), &event, &ack)
            .unwrap()
            .unwrap();
        let decision =
            Decision::new(request, &event, &ack, choice, 999).unwrap_or_else(|_| panic!("prepare"));
        let body: serde_json::Value = serde_json::from_slice(decision.bytes()).unwrap();
        assert_eq!(body["approved"], choice == Choice::Approve);
        assert_eq!(body["confirmationId"], "confirmation:test");
        assert!(body.get("payload").is_none());
        assert!(!decision.attempted() && !decision.confirmed());
    }
    let request = Request::parse_matching(&raw(request(&event)), &event, &ack)
        .unwrap()
        .unwrap();
    let failed = Decision::new(request, &event, &ack, Choice::Approve, 1000)
        .err()
        .unwrap();
    assert_eq!(failed.error, ConfirmationError::Expired);
    assert_eq!(failed.request.id(), "confirmation:test");
}
