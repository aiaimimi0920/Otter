//! Select 仅 change/commit，使用选项 value 而非 label，不附加文本 draft 表单值。
use super::*;
#[test]
fn surface_select_event_preserves_string_value_and_binding_without_input_phase() {
    let scene = Scene::parse(br#"{"id":"select","type":"select","props":{"options":[{"value":"chosen","label":"Label"}]},"events":{"change":"commit","input":"unused"}}"#).unwrap();
    let options = crate::surface_select::Options::parse(scene.root()).unwrap();
    let value = Value::Text(options.value_at(0).unwrap());
    let event = encode(scene.root(), &binding(), 8, Phase::Change, value)
        .unwrap()
        .unwrap();
    let wire: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
    assert_eq!(wire["event"], "change");
    assert_eq!(wire["class"], "commit");
    assert_eq!(wire["payload"], serde_json::json!({"value":"chosen"}));
    assert_eq!(wire["baseRevision"], 8);
    assert_eq!(wire["generation"], 7);
    assert!(wire.get("modifiers").is_none());
    assert!(!event.attempted());
    assert!(encode(scene.root(), &binding(), 8, Phase::Input, value)
        .unwrap()
        .is_none());
    for wrong in [Value::Number(Some(1.)), Value::Boolean(true)] {
        assert!(matches!(
            encode(scene.root(), &binding(), 8, Phase::Change, wrong),
            Err(Error::Value)
        ));
    }
}
#[test]
fn surface_select_event_rejects_unsafe_text_and_suppresses_disabled_hidden_or_missing_action() {
    let select = node("select");
    for value in [
        "bad\0value".to_owned(),
        "x".repeat(crate::surface_input_draft::MAX_VALUE_BYTES + 1),
    ] {
        assert!(matches!(
            encode(
                select.root(),
                &binding(),
                1,
                Phase::Change,
                Value::Text(&value)
            ),
            Err(Error::Value)
        ));
    }
    for props in [
        serde_json::json!({"disabled":true}),
        serde_json::json!({"visible":false}),
    ] {
        let scene = Scene::parse(&serde_json::to_vec(&serde_json::json!({"id":"s","type":"select","props":props,"events":{"change":"run"}})).unwrap()).unwrap();
        assert!(
            encode(scene.root(), &binding(), 1, Phase::Change, Value::Text("x"))
                .unwrap()
                .is_none()
        );
    }
    let scene = Scene::parse(br#"{"id":"s","type":"select"}"#).unwrap();
    assert!(
        encode(scene.root(), &binding(), 1, Phase::Change, Value::Text("x"))
            .unwrap()
            .is_none()
    );
}
