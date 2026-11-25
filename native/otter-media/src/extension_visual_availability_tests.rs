//! Mainline renderer transitions, missing scenes, and offline placeholders using only synthetic validated catalogs/attachments.
use super::*;
use crate::unit_attachment_store::tests as f;
use serde_json::json;

fn without_ocr_renderer() -> Value {
    let mut c = tests::catalog_value();
    c["contributions"]["renderers"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    c
}
fn snapshot(value: &Value) -> Snapshot {
    Snapshot::parse(&serde_json::to_vec(value).unwrap()).unwrap()
}
pub fn text_store() -> Store {
    let mut a = f::attachment();
    a["overlayVisible"] = true.into();
    a["payload"]["surfaceScene"] = json!({"id":"ocr-text","type":"text",
        "layout":{"width":"100%","height":32},"props":{"text":"原文\n第二行","selectable":true},
        "style":{"whiteSpace":"pre"},"events":{"click":"neuro.official/ocr.copy-block"}});
    a["payloadDigest"] = crate::unit_attachment_json::digest(&a["payload"]).into();
    f::store(vec![a])
}
#[test]
fn active_official_ocr_keeps_its_attachment_scene_without_a_matching_renderer() {
    let c = snapshot(&without_ocr_renderer());
    let mut store = text_store();
    let visual = project(Some(&c), &store, 4);
    assert_eq!(visual.len(), 1);
    assert_eq!(visual[0].descriptor, "neuro.official/ocr.result-overlay");
    assert_eq!(visual[0].scene["props"]["text"], "原文\n第二行");
    assert!(visual[0].local_copy && !visual[0].placeholder);
    assert_eq!(visual[0].generation, 0);
    assert_eq!(visual[0].bounds.resolve([100., 80.]), [0., 0., 100., 80.]);
    assert!(toggle(&c, &mut store, 4, false).unwrap());
    let hidden = project(Some(&c), &store, 4);
    // Mainline hidden renderer-less attachments remain in unavailable-data inventories, never misreported as active OCR.
    assert!(hidden.iter().all(|v| v.placeholder));
    assert!(toggle(&c, &mut store, 4, false).unwrap());
    assert_eq!(
        project(Some(&c), &store, 4)[0].scene["props"]["visible"],
        true
    );
}
#[test]
fn declared_renderer_wins_and_fallback_does_not_accept_foreign_types_or_revoked_scopes() {
    let store = text_store();
    assert_eq!(
        project(Some(&tests::catalog()), &store, 4)[0].descriptor,
        "ocr-renderer"
    );
    for trust in ["revoked", "untrusted"] {
        let mut c = without_ocr_renderer();
        c["plugins"][0]["trustStatus"] = trust.into();
        let visual = project(Some(&snapshot(&c)), &store, 4);
        assert!(visual
            .iter()
            .all(|v| v.placeholder && !v.local_copy && v.command.is_none()));
        assert!(!serde_json::to_string(&visual[0].scene)
            .unwrap()
            .contains("第二行"));
    }
    let mut c = without_ocr_renderer();
    c["contributions"]["dataTypes"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert!(project(Some(&snapshot(&c)), &store, 4)
        .iter()
        .all(|v| v.placeholder));
}
#[test]
fn valid_descriptor_with_missing_or_rejected_scene_displays_short_unavailable_text() {
    let mut c = tests::catalog_value();
    c["contributions"]["renderers"][0]["payload"]["payload"]["attachmentScenePath"] =
        "/absent".into();
    let projected = project(Some(&snapshot(&c)), &text_store(), 4);
    assert_eq!(projected[0].scene["props"]["text"], "扩展视图不可用");
    assert!(!projected[0].placeholder);
    c["contributions"]["renderers"][0]["payload"]["payload"]["scene"] =
        json!({"id":"static","type":"text","props":{"text":"Static declared fallback"}});
    assert_eq!(
        project(Some(&snapshot(&c)), &text_store(), 4)[0].scene["props"]["text"],
        "Static declared fallback"
    );
    let mut a = serde_json::to_value(&text_store().items()[0]).unwrap();
    a["payload"]["surfaceScene"]["type"] = "script".into();
    a["payloadDigest"] = crate::unit_attachment_json::digest(&a["payload"]).into();
    let visual = project(
        Some(&snapshot(&without_ocr_renderer())),
        &f::store(vec![a]),
        4,
    );
    assert_eq!(visual[0].scene["props"]["text"], "扩展视图不可用");
    assert!(visual[0].scene.get("events").is_none());
}
#[test]
fn offline_placeholders_are_bounded_noninteractive_and_do_not_hide_or_mutate_attachments() {
    let original = serde_json::to_value(&text_store().items()[0]).unwrap();
    let mut values = Vec::new();
    for index in 0..12 {
        let mut a = original.clone();
        a["attachmentId"] = format!("neuro.official/ocr.retained-{index}").into();
        a["overlayVisible"] = false.into();
        values.push(a);
    }
    let store = f::store(values);
    assert!(store.available());
    let before = serde_json::to_value(store.items()).unwrap();
    let projected = project(None, &store, 4);
    assert_eq!(projected.len(), 1);
    assert!(projected[0].placeholder && !projected[0].local_copy && projected[0].command.is_none());
    assert_eq!(projected[0].scene["children"].as_array().unwrap().len(), 8);
    assert_eq!(projected[0].bounds.resolve([100., 80.]), [6., 6., 88., 68.]);
    assert_eq!(serde_json::to_value(store.items()).unwrap(), before);
}
