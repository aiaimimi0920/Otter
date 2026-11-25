//! Select wire 解释及资源边界；不把数据模型测试当作 HWND/浏览器行为证明。
use super::*;
use crate::surface_scene::Scene;
use serde_json::json;
fn scene(props: Value) -> Scene {
    Scene::parse(
        &serde_json::to_vec(&json!({"id":"select","type":"select","props":props})).unwrap(),
    )
    .unwrap()
}
#[test]
fn surface_select_preserves_order_duplicates_labels_and_strict_strings() {
    let scene = scene(
        json!({"value":"b","options":[{"value":"a","label":"中文🙂"},{"value":"b","label":"Second"},{"value":"b","label":"Duplicate"}, {"value":7,"label":true}, "plain", null]}),
    );
    let options = Options::parse(scene.root()).unwrap();
    assert_eq!(options.source(), "b");
    assert_eq!(options.source_index(), Some(1));
    assert_eq!(options.items().len(), 6);
    assert_eq!(options.items()[0].label(), "中文🙂");
    assert_eq!(options.items()[2].label(), "Duplicate");
    assert_eq!(options.value_at(2), Some("b"));
    for i in 3..6 {
        assert_eq!(options.value_at(i), Some(""));
        assert_eq!(options.items()[i].label(), "");
    }
    assert_eq!(options.value_at(usize::MAX), None);
}
#[test]
fn surface_select_missing_and_non_array_options_are_empty() {
    for props in [
        json!({}),
        json!({"value":5,"options":{}}),
        json!({"options":null}),
    ] {
        let scene = scene(props);
        let options = Options::parse(scene.root()).unwrap();
        assert_eq!(options.source(), "");
        assert!(options.items().is_empty());
        assert_eq!(options.source_index(), None);
    }
    let scene = scene(json!({"value":"missing","options":[{"value":"first"}]}));
    let options = Options::parse(scene.root()).unwrap();
    assert_eq!(options.source(), "missing");
    assert_eq!(options.source_index(), None);
}
#[test]
fn surface_select_rejects_nul_overlong_text_and_excessive_options_without_truncation() {
    for props in [
        json!({"value":"a\u{0}b"}),
        json!({"options":[{"label":"a\u{0}b"}]}),
        json!({"options":[{"value":"a\u{0}b"}]}),
    ] {
        assert!(matches!(
            Options::parse(scene(props).root()),
            Err(Error::Text)
        ));
    }
    assert!(Options::parse(scene(json!({"value":"x".repeat(MAX_TEXT_BYTES)})).root()).is_ok());
    assert!(matches!(
        Options::parse(scene(json!({"value":"x".repeat(MAX_TEXT_BYTES + 1)})).root()),
        Err(Error::Limit)
    ));
    assert!(Options::parse(scene(json!({"options":vec![json!({}); MAX_OPTIONS]})).root()).is_ok());
    assert!(matches!(
        Options::parse(scene(json!({"options":vec![json!({}); MAX_OPTIONS + 1]})).root()),
        Err(Error::Limit)
    ));
    let wrong = Scene::parse(br#"{"id":"x","type":"input"}"#).unwrap();
    assert!(matches!(Options::parse(wrong.root()), Err(Error::Kind)));
}
