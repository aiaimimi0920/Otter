//! 真实 wire 合同，不发送网络请求，不把编码成功当授权或控件挂载成功。
use super::*;
use crate::surface_scene::Scene;
#[path = "surface_select_event_tests.rs"]
mod select;
#[path = "surface_slider_event_tests.rs"]
mod slider;
fn binding() -> Binding {
    Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":7},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":3}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn node(kind: &str) -> Scene {
    Scene::parse(format!(r#"{{"id":"field","type":"{kind}","events":{{"input":"typing","change":"commit"}},"props":{{"eventPayload":{{"ignored":true}}}}}}"#).as_bytes()).unwrap()
}

#[test]
fn surface_switch_change_encodes_only_strict_boolean_and_commit_class() {
    let scene = node("switch");
    for value in [false, true] {
        let event = encode(
            scene.root(),
            &binding(),
            12,
            Phase::Change,
            Value::Boolean(value),
        )
        .unwrap()
        .unwrap();
        let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
        assert_eq!(wire["event"], "change");
        assert_eq!(wire["class"], "commit");
        assert_eq!(wire["payload"], serde_json::json!({"value":value}));
        assert_eq!(wire["baseRevision"], 12);
        assert!(wire.get("modifiers").is_none());
        assert!(!event.attempted());
    }
    assert!(encode(
        scene.root(),
        &binding(),
        12,
        Phase::Input,
        Value::Boolean(true)
    )
    .unwrap()
    .is_none());
    assert!(matches!(
        encode(
            scene.root(),
            &binding(),
            12,
            Phase::Change,
            Value::Text("true")
        ),
        Err(Error::Value)
    ));
    assert!(matches!(
        encode(
            node("input").root(),
            &binding(),
            12,
            Phase::Change,
            Value::Boolean(true)
        ),
        Err(Error::Value)
    ));
    let disabled = Scene::parse(
        br#"{"id":"s","type":"switch","props":{"disabled":true},"events":{"change":"run"}}"#,
    )
    .unwrap();
    assert!(encode(
        disabled.root(),
        &binding(),
        12,
        Phase::Change,
        Value::Boolean(true)
    )
    .unwrap()
    .is_none());
}
#[test]
fn surface_input_event_uses_correct_phase_value_and_immutable_identity() {
    let scene = node("input");
    for (phase, name, class) in [
        (Phase::Input, "input", "continuous"),
        (Phase::Change, "change", "commit"),
    ] {
        let event = encode(scene.root(), &binding(), 12, phase, Value::Text("中文🙂"))
            .unwrap()
            .unwrap();
        let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
        assert_eq!(wire["event"], name);
        assert_eq!(wire["class"], class);
        assert_eq!(wire["baseRevision"], 12);
        assert_eq!(wire["generation"], 7);
        assert_eq!(wire["payload"], serde_json::json!({"value":"中文🙂"}));
        assert!(wire.get("modifiers").is_none());
        assert!(!event.attempted());
        assert_eq!(event.binding(), &binding());
    }
}
#[test]
fn surface_input_event_number_null_and_wrong_types_are_not_silently_coerced() {
    let scene = node("number");
    for value in [None, Some(-1.25)] {
        let event = encode(
            scene.root(),
            &binding(),
            1,
            Phase::Input,
            Value::Number(value),
        )
        .unwrap()
        .unwrap();
        let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
        assert_eq!(wire["payload"]["value"], serde_json::json!(value));
    }
    assert!(matches!(
        encode(scene.root(), &binding(), 1, Phase::Input, Value::Text("1")),
        Err(Error::Value)
    ));
    assert!(matches!(
        encode(
            scene.root(),
            &binding(),
            1,
            Phase::Input,
            Value::Number(Some(f64::INFINITY))
        ),
        Err(Error::Value)
    ));
}
#[test]
fn surface_input_event_disabled_missing_handlers_and_escaped_budget() {
    let disabled = Scene::parse(
        br#"{"id":"field","type":"input","props":{"disabled":true},"events":{"input":"run"}}"#,
    )
    .unwrap();
    assert!(encode(
        disabled.root(),
        &binding(),
        1,
        Phase::Input,
        Value::Text("x")
    )
    .unwrap()
    .is_none());
    let missing = Scene::parse(br#"{"id":"field","type":"input"}"#).unwrap();
    assert!(encode(
        missing.root(),
        &binding(),
        1,
        Phase::Input,
        Value::Text("x")
    )
    .unwrap()
    .is_none());
    let scene = node("textarea");
    assert!(matches!(
        encode(
            scene.root(),
            &binding(),
            1,
            Phase::Input,
            Value::Text(&"\u{0001}".repeat(12000))
        ),
        Err(Error::Encoding(surface_click_event::Error::Encoding))
    ));
    assert!(matches!(
        encode(scene.root(), &binding(), 0, Phase::Input, Value::Text("x")),
        Err(Error::Encoding(surface_click_event::Error::Revision))
    ));
}
