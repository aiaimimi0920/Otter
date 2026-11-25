//! Match mainline OCR/translation-cache visibility, provenance receipts, and scene allowlists using synthetic catalogs/attachments only.
use super::*;
use crate::{unit_attachment_json, unit_attachment_store::tests as f};
use serde_json::json;
pub fn catalog_value() -> Value {
    let mut v = crate::loom_extension_snapshot::tests::fixture();
    let mut translation = v["plugins"][0].clone();
    translation["id"] = TRANSLATE.into();
    translation["scopeId"] = "scope:translation".into();
    v["plugins"].as_array_mut().unwrap().push(translation);
    v["contributions"]["commands"].as_array_mut().unwrap().push(json!({
        "id":"translation-overlay", "commandId":"neuro.official/text-translation.toggle-overlay",
        "pluginId":TRANSLATE, "scopeId":"scope:translation", "when":"unit.kind == 'sticker' && unit.hasImage",
        "payload":{"permissions":["hook.unit.attachments.read","hook.unit.attachments.write"]}
    }));
    v["contributions"]["dataTypes"] = json!([
        {"id":OCR_RESULT,"pluginId":OCR,"scopeId":"scope:ocr"},
        {"id":TRANSLATION,"pluginId":TRANSLATE,"scopeId":"scope:translation"}]);
    let renderer = |id: &str, plugin: &str, scope: &str, type_id: &str, source: bool| {
        let mut p = json!({"typeId":type_id,"bounds":{"x":4,"y":6,"width":32,"height":20},"attachmentScenePath":"/surfaceScene"});
        if source {
            p["sourceContext"] = "ocr-text.v1".into();
            p["replacesSource"] = true.into();
        }
        json!({"id":id,"pluginId":plugin,"scopeId":scope,"payload":{"schema":"renderer.v1","payload":p}})
    };
    v["contributions"]["renderers"] = json!([
        renderer("ocr-renderer", OCR, "scope:ocr", OCR_RESULT, false),
        renderer(
            "translation-renderer",
            TRANSLATE,
            "scope:translation",
            TRANSLATION,
            true
        )
    ]);
    v
}
pub fn catalog() -> Snapshot {
    Snapshot::parse(&serde_json::to_vec(&catalog_value()).unwrap()).unwrap()
}
pub fn attachments() -> Store {
    let mut a = f::attachment();
    a["payload"]["surfaceScene"] = json!({"id":"ocr-root","type":"stack","layout":{"width":32,"height":20},"style":{"background":"#ff0000"},"props":{"visible":false}});
    a["payload"]["visible"] = false.into();
    a["overlayVisible"] = true.into();
    let digest = unit_attachment_json::digest(&a["payload"]);
    a["payloadDigest"] = digest.clone().into();
    let b = json!({"attachmentId":"neuro.official/text-translation.result","typeId":TRANSLATION,"schemaVersion":"1","revision":1,"pluginId":TRANSLATE,"pluginVersion":"1.0.0","payload":{
        "sourceRevision":4,"sourceAttachment":{"attachmentId":"neuro.official/ocr.result","revision":1,"digest":digest},"originalText":"示例","visible":false,
        "surfaceScene":{"id":"translation-root","type":"text","props":{"text":"Example"}}},"resourceRefs":[]});
    f::store(vec![a, b])
}
#[test]
fn replacement_suppresses_source_without_erasing_intent_and_showing_source_dismisses_replacement() {
    let c = catalog();
    let mut s = attachments();
    let original = unit_attachment_json::digest(s.items()[0].attachment.payload().unwrap());
    let p = project(Some(&c), &s, 4);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].plugin, OCR);
    assert_eq!(p[0].scene["props"]["visible"], true);
    assert!(toggle(&c, &mut s, 4, true).unwrap());
    let p = project(Some(&c), &s, 4);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].plugin, TRANSLATE);
    assert_eq!(s.items()[0].visible, Some(true));
    toggle(&c, &mut s, 4, true).unwrap();
    assert_eq!(project(Some(&c), &s, 4)[0].plugin, OCR);
    toggle(&c, &mut s, 4, true).unwrap();
    toggle(&c, &mut s, 4, false).unwrap();
    assert_eq!(project(Some(&c), &s, 4)[0].plugin, OCR);
    assert_eq!(s.items()[1].visible, Some(false));
    assert_eq!(
        unit_attachment_json::digest(s.items()[0].attachment.payload().unwrap()),
        original
    );
}
#[test]
fn replacement_suppresses_ocr_only_if_its_visual_fits() {
    let c = catalog();
    let s = attachments();
    let ocr = serde_json::to_value(&s.items()[0]).unwrap();
    let mut translation = serde_json::to_value(&s.items()[1]).unwrap();
    translation["overlayVisible"] = true.into();
    for (fillers, valid_scene, expected) in
        [(31, true, TRANSLATE), (32, true, OCR), (30, false, OCR)]
    {
        let mut translation = translation.clone();
        if !valid_scene {
            translation["payload"]["surfaceScene"]["type"] = "script".into();
        }
        let mut items = vec![ocr.clone()];
        items.extend((0..fillers).map(|i| {
            let mut filler = ocr.clone();
            filler["attachmentId"] = format!("{OCR}.filler-{i}").into();
            filler
        }));
        items.push(translation.clone());
        let store = f::store(items);
        assert_eq!(store.items().len(), fillers + 2);
        let visuals = project(Some(&c), &store, 4);
        assert_eq!(visuals.len(), 32);
        assert_eq!(
            visuals
                .iter()
                .any(|v| v.attachment == "neuro.official/ocr.result"),
            expected == OCR,
            "fillers: {fillers}"
        );
        assert_eq!(
            visuals
                .iter()
                .any(|v| v.plugin == TRANSLATE && v.scene["id"] == "translation-root"),
            expected == TRANSLATE
        );
        if !valid_scene {
            assert_eq!(visuals.last().unwrap().scene["id"], "extension-unavailable");
        }
    }
}
#[test]
fn stale_revision_digest_text_or_attachment_revision_prevents_translation() {
    let c = catalog();
    let s = attachments();
    assert!(!cached(&c, &s, 5, true));
    for key in ["sourceRevision", "originalText", "sourceAttachment"] {
        let mut values: Vec<_> = s
            .items()
            .iter()
            .map(|a| serde_json::to_value(a).unwrap())
            .collect();
        values[1]["payload"][key] = Value::Null;
        values[1]["overlayVisible"] = true.into();
        let s = f::store(values);
        assert!(!cached(&c, &s, 4, true));
        assert!(project(Some(&c), &s, 4)
            .iter()
            .all(|v| v.plugin != TRANSLATE));
    }
}
#[test]
fn revoked_or_missing_declaration_never_projects_cached_visuals_or_executes_fallback() {
    let s = attachments();
    for trust in ["revoked", "untrusted"] {
        let mut v = catalog_value();
        v["plugins"][0]["trustStatus"] = trust.into();
        v["plugins"][1]["trustStatus"] = trust.into();
        let c = Snapshot::parse(&serde_json::to_vec(&v).unwrap()).unwrap();
        let projected = project(Some(&c), &s, 4);
        assert_eq!(projected.len(), 1);
        assert!(projected[0].placeholder);
        assert!(!projected[0].local_copy);
        assert!(projected[0].command.is_none());
        assert!(!cached(&c, &s, 4, false));
    }
    assert!(!toggle(&catalog(), &mut Store::default(), 4, false).unwrap());
}
#[test]
fn scene_whitelist_depth_duplicate_ids_events_paths_and_bytes_are_bounded() {
    let command = Some("allowed");
    let mut value = json!({"id":"root","type":"column","events":{"click":"unrelated"},"children":[{"id":"button","type":"button","events":{"click":"allowed"}}]});
    let clean = scene::sanitize(&value, command, false, None).unwrap();
    assert!(clean.get("events").is_none());
    assert_eq!(clean["children"][0]["events"]["click"], "allowed");
    for kind in ["image", "input", "scroll", "script", "grid"] {
        value["type"] = kind.into();
        assert!(scene::sanitize(&value, command, true, None).is_none());
    }
    value["type"] = "column".into();
    value["children"][0]["id"] = "root".into();
    assert!(scene::sanitize(&value, command, true, None).is_none());
    assert!(scene::sanitize(
        &json!({"id":"large","type":"text","props":{"text":"x".repeat(16384)}}),
        None,
        false,
        None
    )
    .is_none());
    assert!(scene::sanitize(
        &json!({"id":"large","type":"text","props":{"text":"x".repeat(16384)}}),
        None,
        true,
        None
    )
    .is_some());
    for path in [
        json!("/../surfaceScene"),
        json!("surfaceScene"),
        json!("/a/b/c/d/e/f/g/h/i"),
        json!(null),
    ] {
        assert!(!valid_path(&path));
    }
}
#[test]
fn local_text_copy_requires_the_exact_official_plugin_type_and_schema_combination() {
    for (kind, schema, expected) in [
        (OCR_RESULT, "1", true),
        (OCR_RESULT, "2", false),
        (TRANSLATION, "1", false),
    ] {
        let mut a = f::attachment();
        a["typeId"] = kind.into();
        a["schemaVersion"] = schema.into();
        a["overlayVisible"] = true.into();
        a["payload"]["surfaceScene"] =
            json!({"id":"text","type":"text","props":{"text":"Only display","selectable":true}});
        a["payloadDigest"] = unit_attachment_json::digest(&a["payload"]).into();
        let mut c = catalog_value();
        c["contributions"]["dataTypes"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        c["contributions"]["renderers"]
            .as_array_mut()
            .unwrap()
            .truncate(1);
        c["contributions"]["dataTypes"][0]["id"] = kind.into();
        c["contributions"]["renderers"][0]["payload"]["payload"]["typeId"] = kind.into();
        let c = Snapshot::parse(&serde_json::to_vec(&c).unwrap()).unwrap();
        let visuals = project(Some(&c), &f::store(vec![a]), 4);
        assert_eq!(visuals.len(), 1);
        assert_eq!(visuals[0].local_copy, expected);
        assert_eq!(visuals[0].generation, c.generation());
        assert_eq!(visuals[0].revision, 1);
    }
}
