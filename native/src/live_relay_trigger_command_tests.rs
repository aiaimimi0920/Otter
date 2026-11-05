//! 独立wire构造的合同与单槽围栏，不以生产序列化结果作为预期值。
use super::*;
use crate::live_relay_viewer_trigger_queue::{Phase, Queue};
use serde_json::{json, Value};
fn value() -> Value {
    json!({"bindingId":"binding:progress","enabled":true,
        "target":{"surfaceInstanceId":"surface:real","surfaceAttachmentId":"attachment:real",
            "surfaceNodeId":"finish","surfaceEvent":"click","surfaceAction":"finish"},
        "condition":{"conditionId":"condition:progress","revision":1,"observationId":"uia:edit",
            "operator":"equals","operand":{"path":"/value/text","value":"done"},
            "stableForMs":1000,"risingEdge":false,"rearm":true,"minimumConfidence":"exact"}})
}
fn command() -> Command {
    Command::parse(&value().to_string()).unwrap()
}
#[test]
fn full_contract_roundtrips_without_losing_types_and_rejects_invalid_fields() {
    let command = command();
    assert_eq!(serde_json::to_value(&command).unwrap(), value());
    for (path, value) in [
        ("/condition/revision", json!(0)),
        ("/condition/operand", json!({"path":"/","value":true})),
        ("/condition/stableForMs", json!(86_400_001)),
        ("/target/surfaceAction", json!("bad id")),
    ] {
        let mut invalid = self::value();
        *invalid.pointer_mut(path).unwrap() = value;
        assert!(Command::parse(&invalid.to_string()).is_err(), "{path}");
    }
    let mut numeric = value();
    numeric["condition"]["operator"] = json!("greater_than");
    assert!(Command::parse(&numeric.to_string()).is_err());
    let mut extra = value();
    extra["credentials"] = json!("not-allowed");
    assert!(Command::parse(&extra.to_string()).is_err());
    assert!(Command::parse(&" ".repeat(MAX_TEXT + 1)).is_err());
    let mut huge = value();
    huge["condition"]["operand"] = json!(vec![0; 513]);
    assert!(Command::parse(&huge.to_string()).is_err());
}
#[test]
fn update_and_disable_increment_revision_without_reauthorizing_or_changing_target() {
    let original = command().registration("viewer:one");
    let update = Command::update(&original, false).unwrap();
    assert_eq!(update.condition.revision, 2);
    assert!(update.enabled);
    assert_eq!(update.target, original.target);
    let disabled = Command::update(&original, true).unwrap();
    assert!(!disabled.enabled);
    assert_eq!(disabled.condition.revision, 2);
    assert!(!command().matches(&original, "viewer:other"));
    let mut overflow = original;
    overflow.condition.revision = u64::MAX;
    assert!(Command::update(&overflow, true).is_none());
}
#[test]
fn trigger_queue_is_single_inflight_bound_to_actual_surface_and_stop_is_terminal() {
    let q = Queue::default();
    assert!(!q.submit(command()));
    q.configure(Some(("surface:real".into(), "attachment:real".into())));
    let mut forged = command();
    forged.target.surface_attachment_id = "attachment:other".into();
    assert!(!q.submit(forged));
    assert!(q.submit(command()));
    assert!(!q.submit(command()));
    assert!(q.take().is_some());
    assert!(!q.submit(command()));
    assert!(q.take().is_none());
    assert!(q.complete(Phase::Confirmed));
    assert!(q.submit(command()));
    q.stop();
    assert!(q.take().is_none());
    assert!(!q.available());
    assert!(!q.complete(Phase::Confirmed));
    assert!(!q.submit(command()));
    q.configure(Some(("surface:real".into(), "attachment:real".into())));
    assert!(!q.available());
}
#[test]
fn unknown_result_blocks_replay_and_local_art_without_surface_never_configures() {
    let q = Queue::default();
    q.configure(None);
    assert!(!q.submit(command()));
    q.configure(Some(("surface:real".into(), "attachment:real".into())));
    assert!(q.submit(command()));
    assert!(q.take().is_some());
    q.complete(Phase::Unknown);
    assert!(!q.submit(command()));
}
