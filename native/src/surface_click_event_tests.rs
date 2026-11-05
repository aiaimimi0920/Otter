//! wire 字段、payload 默认规则和输出预算；不发送网络请求。
use super::*;
#[path = "surface_convergence_tests.rs"]
mod convergence;
use crate::{surface_click_route::ClickRoute, surface_scene::Scene};
fn binding() -> Binding {
    Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":7},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":3}}}}"#,
        "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn scene(kind: &str, payload: serde_json::Value) -> Scene {
    Scene::parse(
        &serde_json::to_vec(&serde_json::json!({
            "id":"target", "type":kind, "events":{"click":"run"},
            "props":{"eventPayload":payload,"includeSurfaceValues":true}
        }))
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn container_event_preserves_identity_revision_modifiers_and_unique_event_id() {
    let scene = scene("view", serde_json::json!({"value":"你好"}));
    let action = ClickRoute::new(&scene, Some("target")).next().unwrap();
    let modifiers = Modifiers {
        ctrl_key: true,
        shift_key: true,
        ..Default::default()
    };
    let event = action.encode(&binding(), 57, modifiers).unwrap();
    let next = action.encode(&binding(), 57, modifiers).unwrap();
    assert_ne!(event.id(), next.id());
    let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    assert_eq!(
        wire,
        serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":"instance:test",
            "attachmentId":"attachment:test","eventId":event.id(),"nodeId":"target",
            "event":"click","action":"run","class":"discrete","generation":7,
            "baseRevision":57,"payload":{"value":"你好"},
            "modifiers":{"altKey":false,"ctrlKey":true,"metaKey":false,"shiftKey":true}
        })
    );
}
#[test]
fn button_omits_modifiers_and_null_payload_defaults_to_object() {
    let scene = scene("button", serde_json::Value::Null);
    let action = ClickRoute::new(&scene, Some("target")).next().unwrap();
    let event = action.encode(&binding(), 1, Modifiers::default()).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    assert!(wire.get("modifiers").is_none());
    assert_eq!(wire["payload"], serde_json::json!({}));
    assert!(matches!(
        action.encode(&binding(), 0, Modifiers::default()),
        Err(Error::Revision)
    ));
}
#[test]
fn primitive_payload_is_preserved_and_oversized_escaped_payload_is_rejected() {
    for payload in [
        serde_json::json!(false),
        serde_json::json!(0),
        serde_json::json!([1, 2]),
    ] {
        let scene = scene("view", payload.clone());
        let action = ClickRoute::new(&scene, Some("target")).next().unwrap();
        let event = action.encode(&binding(), 1, Modifiers::default()).unwrap();
        let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
        assert_eq!(wire["payload"], payload);
    }
    let scene = scene(
        "view",
        serde_json::json!("\u{0001}".repeat(MAX_EVENT_BYTES / 5)),
    );
    let action = ClickRoute::new(&scene, Some("target")).next().unwrap();
    assert!(matches!(
        action.encode(&binding(), 1, Modifiers::default()),
        Err(Error::Encoding)
    ));
}
#[test]
fn bounded_writer_accepts_exact_limit_and_rejects_further_data() {
    let mut output = Bounded(Vec::new());
    output.write_all(&vec![0; MAX_EVENT_BYTES]).unwrap();
    assert_eq!(output.0.len(), MAX_EVENT_BYTES);
    assert!(output.write_all(&[1]).is_err());
    assert_eq!(output.0.len(), MAX_EVENT_BYTES);
}

fn event() -> ClickEvent {
    let scene = scene("view", serde_json::json!({}));
    ClickRoute::new(&scene, Some("target"))
        .next()
        .unwrap()
        .encode(&binding(), 57, Modifiers::default())
        .unwrap()
}
fn ack(event: &ClickEvent, status: &str) -> serde_json::Value {
    serde_json::json!({"protocolVersion":"loom.surface.v1",
        "instanceId":"instance:test","eventId":event.id(),
        "requestId":"request:test","accepted":true,"status":status})
}
#[test]
fn ack_distinguishes_acceptance_confirmation_unknown_and_success() {
    use crate::surface_action_ack::Ack;
    let event = event();
    assert_eq!(event.revision(), 57);
    assert_eq!(event.binding().generation(), 7);
    for status in [
        "accepted",
        "awaiting_confirmation",
        "queued",
        "running",
        "cancel_requested",
        "cancelled",
        "succeeded",
        "failed",
        "interrupted",
        "unknown",
    ] {
        let parsed =
            Ack::parse(&serde_json::to_vec(&ack(&event, status)).unwrap(), &event).unwrap();
        assert_eq!(parsed.succeeded(), status == "succeeded");
        assert_eq!(parsed.request_id(), "request:test");
        assert!(parsed.accepted());
    }
    let mut rejected = ack(&event, "succeeded");
    rejected["accepted"] = false.into();
    assert!(!Ack::parse(&serde_json::to_vec(&rejected).unwrap(), &event)
        .unwrap()
        .succeeded());
}
#[test]
fn ack_rejects_cross_event_or_instance_and_malformed_identity() {
    use crate::surface_action_ack::{Ack, Error};
    let event = event();
    for (key, value) in [
        ("instanceId", "other"),
        ("eventId", "other"),
        ("protocolVersion", "other"),
        ("requestId", ""),
        ("requestId", "bad\n"),
    ] {
        let mut wire = ack(&event, "accepted");
        wire[key] = value.into();
        assert!(matches!(
            Ack::parse(&serde_json::to_vec(&wire).unwrap(), &event),
            Err(Error::Identity)
        ));
    }
    assert!(matches!(
        Ack::parse(
            &serde_json::to_vec(&ack(&event, "new_status")).unwrap(),
            &event
        ),
        Err(Error::Json)
    ));
}
#[test]
fn ack_update_is_atomic_and_preserves_remote_failure_without_claiming_success() {
    use crate::surface_action_ack::{Ack, Status};
    let event = event();
    let mut parsed = Ack::parse(
        &serde_json::to_vec(&ack(&event, "running")).unwrap(),
        &event,
    )
    .unwrap();
    let mut wrong = ack(&event, "succeeded");
    wrong["requestId"] = "request:other".into();
    assert!(parsed
        .update(&serde_json::to_vec(&wrong).unwrap(), &event)
        .is_err());
    assert_eq!(parsed.status(), Status::Running);
    let mut failed = ack(&event, "failed");
    failed["error"] =
        serde_json::json!({"code":"denied","message":"需要确认","detail":"remote text"});
    parsed
        .update(&serde_json::to_vec(&failed).unwrap(), &event)
        .unwrap();
    assert!(!parsed.succeeded());
    assert_eq!(parsed.failure().unwrap().code(), "denied");
    assert_eq!(parsed.failure().unwrap().message(), "需要确认");
    assert_eq!(parsed.failure().unwrap().detail(), Some("remote text"));
}
#[test]
fn ack_budget_and_field_types_are_checked_before_acceptance() {
    use crate::surface_action_ack::{Ack, Error, MAX_ACK_BYTES};
    let event = event();
    assert!(matches!(
        Ack::parse(&vec![b' '; MAX_ACK_BYTES + 1], &event),
        Err(Error::Limit)
    ));
    assert!(matches!(Ack::parse(&[], &event), Err(Error::Limit)));
    let mut wrong = ack(&event, "accepted");
    wrong["accepted"] = "true".into();
    assert!(matches!(
        Ack::parse(&serde_json::to_vec(&wrong).unwrap(), &event),
        Err(Error::Json)
    ));
}
