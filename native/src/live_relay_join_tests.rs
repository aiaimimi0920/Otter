use super::*;
use serde_json::{json, Value};
fn snapshot() -> Value {
    json!({"session":{"protocolVersion":protocol::PROTOCOL,"sessionId":"live:test",
        "sourceDeviceId":"source-1","sourceHookId":"hook-1","revision":4,
        "viewerDevices":["device-1"]},"epoch":2,"closed":false,
        "requesterControl":{"deviceId":"device-1","epoch":2,"controlSequence":8,"inputSequence":3}})
}
#[test]
fn actor_baseline_requires_current_member_identity_epoch_and_bounded_sequences() {
    let good = snapshot();
    let preview = Snapshot::parse(&serde_json::to_vec(&good).unwrap(), "live:test").unwrap();
    let parse = |value: &Value| recovery(&serde_json::to_vec(value).unwrap(), &preview, "device-1");
    assert_eq!(
        parse(&good),
        Ok(RequesterCursor {
            control: 8,
            input: 3
        })
    );
    for (pointer, bad) in [
        ("/requesterControl/deviceId", json!("other")),
        ("/requesterControl/epoch", json!(3)),
        ("/requesterControl/controlSequence", json!(u64::MAX)),
        (
            "/requesterControl/inputSequence",
            json!(crate::live_input::MAX_SEQUENCE),
        ),
        ("/requesterControl/inputSequence", json!(-1)),
        ("/session/revision", json!(3)),
        ("/session/viewerDevices", json!([])),
        ("/session/sourceHookId", json!("changed")),
        ("/epoch", json!(3)),
    ] {
        let mut value = good.clone();
        *value.pointer_mut(pointer).unwrap() = bad;
        assert!(parse(&value).is_err(), "{pointer}");
    }
    let mut missing = good.clone();
    missing.as_object_mut().unwrap().remove("requesterControl");
    assert!(parse(&missing).is_err());
}
#[test]
fn discovery_rejects_ambiguous_target_missing_membership_and_invalid_protocol() {
    let parse = |value: &Value| discovery(&serde_json::to_vec(value).unwrap(), "live:test");
    let good = json!({"protocolVersion":protocol::PROTOCOL,"sessions":[snapshot()]});
    assert!(parse(&good).is_ok());
    let mut duplicate = good.clone();
    duplicate["sessions"]
        .as_array_mut()
        .unwrap()
        .push(snapshot());
    assert!(parse(&duplicate).is_err());
    let mut missing = good.clone();
    missing["sessions"][0]["session"]
        .as_object_mut()
        .unwrap()
        .remove("viewerDevices");
    assert!(parse(&missing).is_err());
    assert!(parse(&json!({"protocolVersion":"other","sessions":[snapshot()]})).is_err());
    assert!(parse(&json!({"protocolVersion":protocol::PROTOCOL,"sessions":[]})).is_err());
}
