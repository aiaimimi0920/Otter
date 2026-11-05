use super::*;
use serde_json::{json, Value};
fn audit(key: &str) -> Value {
    json!({"triggerId":"trigger:1","bindingId":"binding:1","conditionRevision":1,
        "observationId":"obs:1","observationSequence":1,"sourceDeviceId":"source:1",
        "observationSource":"ui_automation","idempotencyKey":key,"outcome":"fired",
        "evaluatedAtMs":1,"authorizedBy":"device:1","actionRequestId":"action:1"})
}
fn encoded(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn reservation(key: &str) -> Value {
    let mut value = audit(key);
    value.as_object_mut().unwrap().remove("actionRequestId");
    value["reason"] = json!("surface_action_dispatch_reserved");
    value
}
#[test]
fn final_audit_replaces_reservation_but_never_regresses() {
    let pending = encoded(&reservation("key:1"));
    let final_value = encoded(&audit("key:1"));
    let mut state = Audits::default();
    state.apply([pending.as_slice()]).unwrap();
    assert!(state.iter().next().unwrap().reservation());
    state
        .apply([
            final_value.as_slice(),
            pending.as_slice(),
            final_value.as_slice(),
        ])
        .unwrap();
    assert_eq!(state.iter().count(), 1);
    assert_eq!(
        state.iter().next().unwrap().action_request_id.as_deref(),
        Some("action:1")
    );
    let restored = Audits::from_snapshot(&encoded(
        &json!({"triggerAudits":[audit("key:1"),reservation("key:1")]}),
    ))
    .unwrap();
    assert!(!restored.iter().next().unwrap().reservation());
}
#[test]
fn invalid_batch_never_commits_partial_audit_changes() {
    let pending = encoded(&reservation("key:1"));
    let final_value = encoded(&audit("key:1"));
    let mut state = Audits::default();
    state.apply([pending.as_slice()]).unwrap();
    assert_eq!(
        state.apply([final_value.as_slice(), b"{}".as_slice()]),
        Err(Error::Json)
    );
    assert!(state.iter().next().unwrap().reservation());
}
#[test]
fn strict_fields_counters_identifiers_and_byte_limits_are_preserved() {
    for (key, value) in [
        ("triggerId", json!("")),
        ("conditionRevision", json!(0)),
        ("observationSequence", json!(0)),
        ("evaluatedAtMs", json!(0)),
        ("actionRequestId", json!("bad id")),
        ("reason", json!("中".repeat(171))),
        ("outcome", json!("reserved")),
        ("invented", json!(true)),
    ] {
        let mut input = audit("key:1");
        input[key] = value;
        assert!(Audit::parse(&encoded(&input)).is_err(), "{key}");
    }
    assert!(Audits::from_snapshot(b"{}").is_ok());
    assert!(Audits::from_snapshot(br#"{"triggerAudits":null}"#).is_err());
}
#[test]
fn snapshot_limit_and_incremental_fifo_retention_are_bounded() {
    let values: Vec<_> = (0..257).map(|i| audit(&format!("key:{i}"))).collect();
    assert!(matches!(
        Audits::from_snapshot(&encoded(&json!({"triggerAudits":values}))),
        Err(Error::Limit)
    ));
    let bytes: Vec<_> = values.iter().map(encoded).collect();
    let mut state = Audits::default();
    state.apply(bytes.iter().map(Vec::as_slice)).unwrap();
    assert_eq!(state.iter().count(), 256);
    assert_eq!(state.iter().next().unwrap().idempotency_key, "key:1");
    assert_eq!(state.iter().last().unwrap().idempotency_key, "key:256");
}
