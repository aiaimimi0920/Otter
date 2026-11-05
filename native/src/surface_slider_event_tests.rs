//! Slider 数值 input/continuous 与 change/commit 分离；不进入文本草稿或表单集合。
use super::*;
#[test]
fn surface_slider_event_preserves_number_phase_and_binding() {
    let scene = node("slider");
    for (phase, event, class) in [
        (Phase::Input, "input", "continuous"),
        (Phase::Change, "change", "commit"),
    ] {
        let encoded = encode(
            scene.root(),
            &binding(),
            9,
            phase,
            Value::Number(Some(0.25)),
        )
        .unwrap()
        .unwrap();
        let wire: serde_json::Value = serde_json::from_slice(encoded.bytes()).unwrap();
        assert_eq!(wire["event"], event);
        assert_eq!(wire["class"], class);
        assert_eq!(wire["payload"], serde_json::json!({"value":0.25}));
        assert_eq!(wire["baseRevision"], 9);
        assert_eq!(wire["generation"], 7);
        assert!(wire.get("modifiers").is_none());
        assert!(!encoded.attempted());
    }
    for wrong in [
        Value::Number(None),
        Value::Number(Some(f64::NAN)),
        Value::Text("1"),
        Value::Boolean(true),
    ] {
        assert!(matches!(
            encode(scene.root(), &binding(), 9, Phase::Input, wrong),
            Err(Error::Value)
        ));
    }
}
#[test]
fn surface_slider_event_suppresses_hidden_disabled_and_undeclared_actions() {
    for props in [
        serde_json::json!({"disabled":true}),
        serde_json::json!({"visible":false}),
    ] {
        let scene=Scene::parse(&serde_json::to_vec(&serde_json::json!({"id":"s","type":"slider","props":props,"events":{"input":"move","change":"commit"}})).unwrap()).unwrap();
        for phase in [Phase::Input, Phase::Change] {
            assert!(
                encode(scene.root(), &binding(), 1, phase, Value::Number(Some(1.)))
                    .unwrap()
                    .is_none()
            );
        }
    }
    let scene = Scene::parse(br#"{"id":"s","type":"slider"}"#).unwrap();
    assert!(encode(
        scene.root(),
        &binding(),
        1,
        Phase::Change,
        Value::Number(Some(1.))
    )
    .unwrap()
    .is_none());
}
