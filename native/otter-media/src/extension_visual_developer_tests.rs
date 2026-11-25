//! 主干开发模式的显示/执行分界；正向场景不能替代 trusted 命令和附件写入授权。
use super::*;
use crate::{
    extension_when::{Context, UnitKind},
    unit_attachment_store::tests as f,
};
use serde_json::json;

fn data(trust: &str) -> Value {
    let mut data = tests::catalog_value();
    for plugin in data["plugins"].as_array_mut().unwrap() {
        plugin["trustStatus"] = trust.into();
    }
    data
}
fn parse(data: &Value) -> Snapshot {
    Snapshot::parse(&serde_json::to_vec(data).unwrap()).unwrap()
}
#[test]
fn display_accepts_developer_scopes_but_commands_shortcuts_and_write_declarations_stay_trusted() {
    let context = Context {
        kind: Some(UnitKind::Sticker),
        has_image: true,
        attachment_types: &[],
    };
    for (trust, display, trusted) in [
        ("trusted", true, true),
        ("unsigned_developer", true, false),
        ("revoked", false, false),
        ("untrusted", false, false),
    ] {
        let mut data = data(trust);
        data["contributions"]["shortcuts"] = json!([{
            "id":"developer-shortcut", "pluginId":OCR, "scopeId":"scope:ocr",
            "commandId":"neuro.official/ocr.toggle-overlay", "payload":{"keys":"Ctrl+4"}
        }]);
        let catalog = parse(&data);
        assert_eq!(catalog.display_active("scope:ocr"), display, "{trust}");
        assert!(!catalog.display_active("missing"));
        assert_eq!(
            catalog.visual_declarations("dataTypes").count(),
            if display { 2 } else { 0 }
        );
        assert_eq!(catalog.owner("scope:ocr").is_some(), trusted);
        assert_eq!(
            catalog.declares("scope:ocr", "dataTypes", OCR_RESULT),
            trusted
        );
        assert_eq!(
            catalog
                .command("neuro.official/ocr.toggle-overlay", &context)
                .is_some(),
            trusted
        );
        assert_eq!(!catalog.shortcuts().unwrap().is_empty(), trusted);
        assert_eq!(
            catalog.shortcut_available("developer-shortcut", &context),
            trusted
        );
        let projected = project(Some(&catalog), &tests::attachments(), 4);
        assert_eq!(projected[0].descriptor == "ocr-renderer", display);
        assert_eq!(projected[0].placeholder, !display);
    }
}
#[test]
fn developer_renderer_and_overlay_keep_declared_actions_without_authorizing_execution() {
    let mut data = data("unsigned_developer");
    let command = "neuro.official/ocr.toggle-overlay";
    data["contributions"]["renderers"][0]["commandId"] = command.into();
    let mut overlay = data["contributions"]["renderers"][0].clone();
    overlay["id"] = "developer-overlay".into();
    overlay["payload"]["schema"] = "unit-overlay.v1".into();
    data["contributions"]["unitOverlays"] = json!([overlay]);
    let store = tests::attachments();
    let projected = project(Some(&parse(&data)), &store, 4);
    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].descriptor, "ocr-renderer");
    assert_eq!(projected[1].descriptor, "developer-overlay");
    assert!(projected
        .iter()
        .all(|v| v.command.as_deref() == Some(command)));
    // 声明存在是显示前置条件，不是可执行凭证；未声明时两项均不能冒充有效 descriptor。
    data["contributions"]["commands"] = json!([]);
    let projected = project(Some(&parse(&data)), &store, 4);
    assert!(projected
        .iter()
        .all(|v| !["ocr-renderer", "developer-overlay"].contains(&v.descriptor.as_str())));
}
#[test]
fn developer_cache_toggle_and_official_fallback_preserve_payload_while_effect_writes_remain_denied()
{
    let mut data = data("unsigned_developer");
    data["contributions"]["renderers"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let catalog = parse(&data);
    let mut store = tests::attachments();
    let original = serde_json::to_value(&store.items()[0].attachment).unwrap();
    assert_eq!(
        project(Some(&catalog), &store, 4)[0].descriptor,
        "neuro.official/ocr.result-overlay"
    );
    assert!(cached(&catalog, &store, 4, false));
    assert!(toggle(&catalog, &mut store, 4, false).unwrap());
    assert!(project(Some(&catalog), &store, 4)
        .iter()
        .all(|v| v.placeholder));
    assert!(toggle(&catalog, &mut store, 4, false).unwrap());
    assert_eq!(
        serde_json::to_value(&store.items()[0].attachment).unwrap(),
        original
    );
    let stamp = store.stamp();
    let mut value = f::attachment();
    value["priorRevision"] = 1.into();
    value["revision"] = 2.into();
    assert_eq!(
        store.upsert(&catalog, "scope:ocr", 4, 4, value),
        Err(crate::unit_attachment_store::Error::Owner)
    );
    assert_eq!(
        store.remove(&catalog, "scope:ocr", 4, 4, "neuro.official/ocr.result", 1),
        Err(crate::unit_attachment_store::Error::Owner)
    );
    assert!(stamp.matches(&store));
}
