//! Exact whitelists, boundaries, and configuration for baseline named input; no real models, user resources, or network.
use super::*;
use crate::unit_attachment_store::tests as fixture;
fn attachment() -> Value {
    let mut a = fixture::attachment();
    a["payload"] = json!({"fullText":"示例\ntext", "sourceWidth":64, "sourceHeight":32,
        "textBlocks":[{"text":"示例", "left":0,"top":1,"width":32,"height":20,
            "textColor":"#AbC123","backgroundColor":"bad","secret":"not exported"}],
        "opaque":"not exported"});
    a["payloadDigest"] = crate::unit_attachment_json::digest(&a["payload"]).into();
    a
}
#[test]
fn named_input_exports_only_text_geometry_and_source_identity() {
    let store = fixture::store(vec![attachment()]);
    let input = prepare(&store, 4, "en", "gateway").unwrap().unwrap();
    assert_eq!(input["text"], "示例\ntext");
    assert_eq!(input["targetLanguage"], "en");
    assert_eq!(input["providerMode"], "gateway");
    assert_eq!(input["sourceAttachment"]["revision"], 1);
    assert_eq!(
        input["sourceAttachment"]["digest"],
        store.items()[0].digest.as_deref().unwrap()
    );
    assert_eq!(input["textBlocks"][0]["backgroundColor"], "#111111");
    assert_eq!(input["textBlocks"][0]["textColor"], "#AbC123");
    assert!(!input.to_string().contains("not exported"));
    assert!(prepare(&store, 4, "zh-CN", "auto")
        .unwrap()
        .unwrap()
        .get("providerMode")
        .is_none());
    assert!(prepare(&Store::default(), 4, "en", "auto")
        .unwrap()
        .is_none());
}
#[test]
fn text_and_geometry_are_bounded_before_translation() {
    for case in 0..7 {
        let mut a = attachment();
        a.as_object_mut().unwrap().remove("payloadDigest");
        let p = &mut a["payload"];
        match case {
            0 => p["fullText"] = " ".into(),
            1 => p["fullText"] = "x".repeat(16_385).into(),
            2 => p["sourceWidth"] = 0.into(),
            3 => p["sourceHeight"] = 100_001.into(),
            4 => p["textBlocks"][0]["left"] = (-1).into(),
            5 => p["textBlocks"][0]["width"] = 66.into(),
            _ => p["textBlocks"] = json!([null]),
        }
        assert!(
            prepare(&fixture::store(vec![a]), 4, "en", "auto").is_err(),
            "case {case}"
        );
    }
}
#[test]
fn translation_preferences_match_main_normalization_without_rewriting_unknown_fields() {
    for (language, provider, expected) in [
        ("en", "gateway", ("en", "gateway")),
        ("", "local", ("zh-CN", "local")),
        ("invalid", "invalid", ("zh-CN", "auto")),
    ] {
        let document = crate::app_settings_document::Document::decode(
            &serde_json::to_vec(&json!({
            "translationTargetLanguage":language,"translationProviderMode":provider,"future":true}))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(document.translation_options(), expected);
    }
}
