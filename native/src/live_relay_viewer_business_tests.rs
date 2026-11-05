use super::*;
use serde_json::{json, Value};
fn registration() -> Value {
    json!({"binding":{"bindingId":"b","observationId":"o","conditionRevision":1,"authorizedBy":"d","enabled":true},
        "condition":{"conditionId":"c","revision":1,"observationId":"o","operator":"greater_or_equal",
        "operand":{"path":"/value","value":5},"stableForMs":0,"risingEdge":true,"rearm":true,"minimumConfidence":"high"},
        "target":{"surfaceInstanceId":"s","surfaceAttachmentId":"a","surfaceNodeId":"n","surfaceEvent":"e","surfaceAction":"x"},
        "armed":true,"lastMatch":false,"pendingOfflineMatch":false})
}
fn snapshot() -> Value {
    json!({"session":{"observationCapabilities":["adapter"]},"observations":[],"triggers":[registration()],"triggerAudits":[]})
}
fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn event(kind: &str, payload: Value) -> ControlEvent {
    serde_json::from_value(
        json!({"protocolVersion":"loom.live.v1","sessionId":"session","epoch":1,
        "sequence":1,"messageType":kind,"payload":payload}),
    )
    .unwrap()
}
fn observation() -> Value {
    json!({"observationId":"o","sequence":1,"state":"stable","source":"app_adapter",
        "confidence":"high","observedAtMs":1,"value":7})
}
fn audit() -> Value {
    json!({"triggerId":"t","bindingId":"b","conditionRevision":1,"observationId":"o",
        "observationSequence":1,"sourceDeviceId":"d","observationSource":"app_adapter",
        "idempotencyKey":"k","outcome":"fired","evaluatedAtMs":1,"authorizedBy":"d"})
}
#[test]
fn restores_complete_business_snapshot_and_preserves_registration() {
    let state = Business::from_snapshot(&bytes(&snapshot())).unwrap();
    assert_eq!(state.registrations().len(), 1);
    assert_eq!(state.registrations()[0].binding.binding_id, "b");
    assert_eq!(state.capabilities().values().len(), 1);
    assert_eq!(state.observations().iter().count(), 0);
    assert_eq!(state.audits().iter().count(), 0);
    let mut invalid = snapshot();
    invalid.as_object_mut().unwrap().remove("observations");
    assert!(Business::from_snapshot(&bytes(&invalid)).is_err());
}
#[test]
fn invalid_audit_rolls_back_valid_observation_in_same_batch() {
    let mut state = Business::from_snapshot(&bytes(&snapshot())).unwrap();
    assert!(state
        .apply(&[
            event("observation", observation()),
            event("trigger_event", json!({}))
        ])
        .is_err());
    assert!(state.observations().get("o").is_none());
    state
        .apply(&[
            event("observation", observation()),
            event("trigger_event", audit()),
        ])
        .unwrap();
    assert_eq!(state.observations().get("o").unwrap().sequence, 1);
    assert_eq!(state.audits().iter().count(), 1);
    assert_eq!(state.registrations().len(), 1);
}
#[test]
fn viewer_never_executes_input_or_treats_configuration_as_audit() {
    let mut state = Business::from_snapshot(&bytes(&snapshot())).unwrap();
    state
        .apply(&[
            event("input_event", json!({})),
            event("trigger_condition", json!({})),
            event("session_state", json!({})),
        ])
        .unwrap();
    assert_eq!(state.observations().iter().count(), 0);
    assert_eq!(state.audits().iter().count(), 0);
}
#[test]
fn registrations_validate_binding_numeric_selector_and_operand_bounds() {
    for (path, value) in [
        ("/binding/conditionRevision", json!(2)),
        ("/binding/observationId", json!("other")),
        ("/condition/stableForMs", json!(86_400_001)),
        ("/condition/operand", json!({"path":"/value","value":"5"})),
        ("/condition/operand", json!({"path":"/","value":5})),
        ("/condition/operand", json!(vec![0; 513])),
        ("/target/surfaceNodeId", json!("bad id")),
    ] {
        let mut value_copy = registration();
        *value_copy.pointer_mut(path).unwrap() = value;
        assert!(
            registration::from_snapshot(&bytes(&json!({"triggers":[value_copy]}))).is_err(),
            "{path}"
        );
    }
    let mut deep = registration();
    deep["condition"]["operator"] = json!("equals");
    deep["condition"]["operand"] = (0..17).fold(json!(1), |value, _| json!([value]));
    assert!(registration::from_snapshot(&bytes(&json!({"triggers":[deep]}))).is_err());
    assert!(matches!(
        registration::from_snapshot(&bytes(&json!({"triggers":vec![registration();65]}))),
        Err(registration::Error::Limit)
    ));
}
#[test]
fn registration_refresh_preserves_reliable_observations_and_audits() {
    let mut state = Business::from_snapshot(&bytes(&snapshot())).unwrap();
    state
        .apply(&[
            event("observation", observation()),
            event("trigger_event", audit()),
        ])
        .unwrap();
    let before = state.clone();
    state.replace_registrations(vec![]);
    assert!(!state.same_display(&before));
    assert_eq!(state.observations().get("o").unwrap().sequence, 1);
    assert_eq!(state.audits().iter().count(), 1);
    let unchanged = state.clone();
    state.replace_registrations(vec![]);
    assert!(state.same_display(&unchanged));
    let mut second = observation();
    second["sequence"] = json!(2);
    state.apply(&[event("observation", second)]).unwrap();
    assert_eq!(state.observations().get("o").unwrap().sequence, 2);
}
