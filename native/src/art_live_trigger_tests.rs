//! 独立wire常量与负向矩阵；测试数据不从新解析器反推。
use super::*;
use serde_json::{json, Value};
pub(crate) fn binding() -> Binding {
    Binding {
        origin: "https://localhost:1234".into(),
        request_id: "request:1".into(),
        device_id: "viewer:javascript".into(),
        workspace_id: "workspace:1".into(),
        instance_id: "instance:javascript".into(),
        generation: 7,
        art_id: "vendor/counter".into(),
        art_version: "1.0.0".into(),
        package_digest: "a".repeat(64),
        sdk_version: "otter.art.v1".into(),
    }
}
pub(crate) fn identity() -> Identity {
    crate::live_relay_viewer_identity::Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:test","sourceDeviceId":"source:1","sourceHookId":"hook:1","revision":1,"viewerDevices":["viewer:javascript"]},"epoch":1,"closed":false}"#, "live:test").unwrap().identity().clone()
}
pub(crate) fn condition() -> Condition {
    serde_json::from_value(json!({"conditionId":"condition:1","revision":1,"observationId":"obs:1","operator":"equals",
        "operand":{"path":"/value/text","value":"eight"},"stableForMs":0,"risingEdge":false,"rearm":true,"minimumConfidence":"exact"})).unwrap()
}
pub(crate) fn delivery() -> Delivery {
    let now = ticket::now().unwrap();
    serde_json::from_value(json!({"deliveryId":"delivery:1","callback":"onLiveTrigger","conditionRevision":1,
        "sourceDeviceId":"source:1","expiresAtMs":now+30_000,"observation":{"observationId":"obs:1","sequence":2,
        "state":"stable","source":"app_adapter","confidence":"exact","observedAtMs":now,"stableSinceMs":now,
        "locator":null,"value":{"text":"eight"},"reason":null}})).unwrap()
}
pub(crate) fn ticket(binding: Binding) -> (std::sync::Arc<ticket::Authorization>, ticket::Ticket) {
    let gate = ticket::Authorization::new();
    let ticket = ticket::Ticket::new(
        binding,
        gate.clone(),
        Event {
            session_id: "live:test".into(),
            epoch: 1,
            delivery: delivery(),
        },
    )
    .unwrap();
    (gate, ticket)
}
fn reply() -> Value {
    json!({"protocolVersion":PROTOCOL,"binding":binding(),"grantId":"grant:1","scope":{"sessionId":"live:test","epoch":1,
        "bindingId":"local-art:instance:javascript","authorizationId":"auth:1"},"condition":condition(),"enabled":true,
        "status":"claimed","reason":null,"deliveryId":"delivery:1","receipt":null,"delivery":delivery()})
}
fn parse(v: &Value) -> bool {
    let scope = serde_json::from_value(reply()["scope"].clone()).unwrap();
    Reply::parse(
        &serde_json::to_vec(v).unwrap(),
        &binding(),
        "grant:1",
        &scope,
        &condition(),
        &identity(),
    )
    .is_ok()
}
#[test]
fn reply_rejects_identity_scope_revision_source_and_unknown_fields() {
    let original = reply();
    assert!(parse(&original));
    for (pointer, value) in [
        ("/binding/workspaceId", json!("workspace:old")),
        ("/grantId", json!("grant:old")),
        ("/scope/epoch", json!(2)),
        ("/scope/authorizationId", json!("auth:old")),
        ("/condition/revision", json!(2)),
        ("/delivery/conditionRevision", json!(2)),
        ("/delivery/sourceDeviceId", json!("source:other")),
        ("/delivery/observation/observationId", json!("obs:other")),
        ("/delivery/callback", json!("click")),
        ("/deliveryId", json!("delivery:other")),
        ("/delivery/observation/sequence", json!(0)),
        ("/delivery/observation/sequence", json!(MAX_SEQUENCE + 1)),
    ] {
        let mut value0 = original.clone();
        *value0.pointer_mut(pointer).unwrap() = value;
        assert!(!parse(&value0), "{pointer}");
    }
    let mut extra = original;
    extra["delivery"]["grant"] = json!("forged");
    assert!(!parse(&extra));
}
#[test]
fn reply_state_and_delivery_budgets_are_strict_but_unknown_claim_has_no_payload() {
    let original = reply();
    for (status, enabled, receipt, accepted) in [
        ("claimed", true, Value::Null, true),
        ("ready", true, Value::Null, false),
        ("idle", true, Value::Null, false),
        ("claimed", false, Value::Null, false),
        (
            "acknowledged",
            true,
            json!({"kind":"applied","formalRevision":1}),
            false,
        ),
    ] {
        let mut r = original.clone();
        r["status"] = json!(status);
        r["enabled"] = json!(enabled);
        r["receipt"] = receipt;
        assert_eq!(parse(&r), accepted, "{status}");
    }
    let mut r = original.clone();
    r["delivery"] = Value::Null;
    assert!(parse(&r));
    r["status"] = json!("acknowledged");
    r["receipt"] = json!({"kind":"applied","formalRevision":0});
    assert!(!parse(&r));
    r["receipt"] = json!({"kind":"applied","formalRevision":1});
    assert!(parse(&r));
    let mut r = original.clone();
    r["delivery"]["expiresAtMs"] = r["delivery"]["observation"]["observedAtMs"].clone();
    assert!(!parse(&r));
    let mut r = original;
    r["delivery"]["observation"]["value"] = json!("x".repeat(MAX_DELIVERY));
    assert!(!parse(&r));
}
