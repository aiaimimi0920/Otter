//! Cached full text matches mainline: copying works while hidden, translations must match their source, and existing attachments take precedence over legacy results.
use super::*;
use crate::{unit_attachment_json, unit_attachment_store::tests as f};
use serde_json::{json, Value};
pub fn attachments(full: &str, translated: &str) -> Vec<Value> {
    let mut a = f::attachment();
    a["payload"] = json!({"fullText":full,"visible":false});
    a["overlayVisible"] = false.into();
    let digest = unit_attachment_json::digest(&a["payload"]);
    a["payloadDigest"] = digest.clone().into();
    let b = json!({"attachmentId":"neuro.official/text-translation.result", "typeId":visual::TRANSLATION,
        "schemaVersion":"1","revision":1,"pluginId":visual::TRANSLATE,"pluginVersion":"1.0.0",
        "overlayVisible":false,"payload":{"text":translated,"sourceRevision":4,"originalText":full,
        "sourceAttachment":{"attachmentId":a["attachmentId"],"revision":1,"digest":digest}},"resourceRefs":[]});
    vec![a, b]
}
fn legacy(ocr: Value, translation: Value) -> Legacy {
    let raw = |v: Value| RawValue::from_string(v.to_string()).unwrap();
    Legacy::restore(Some(&raw(ocr)), Some(&raw(translation)))
}
#[test]
fn hidden_full_text_is_exact_and_independent_of_renderer_catalog_or_online_state() {
    let full = format!("  中文😀\n{}\nend  ", "text".repeat(20_000));
    let s = f::store(attachments(&full, "  translation\n全文  "));
    let stamp = s.stamp();
    assert_eq!(
        text(Kind::Ocr, &s, None, &Legacy::default()),
        Some(full.as_str())
    );
    assert_eq!(
        text(Kind::Translation, &s, Some(4), &Legacy::default()),
        Some("  translation\n全文  ")
    );
    assert!(stamp.matches(&s));
    assert!(s.items().iter().all(|v| !v.is_visible()));
}
#[test]
fn stale_translation_never_falls_back_to_old_text_but_ocr_cache_remains_available() {
    let fallback = legacy(
        json!({"fullText":"old"}),
        json!({"fullText":"old translated","sourceRevision":4}),
    );
    for case in 0..7 {
        let mut values = attachments("source", "translated");
        let p = &mut values[1]["payload"];
        match case {
            0 => p["sourceRevision"] = 5.into(),
            1 => p["sourceAttachment"]["revision"] = 2.into(),
            2 => p["sourceAttachment"]["digest"] = "wrong".into(),
            3 => p["sourceAttachment"]["attachmentId"] = "other".into(),
            4 => p["originalText"] = "other".into(),
            5 => p["text"] = " \n ".into(),
            _ => {
                values.remove(0);
            }
        }
        let s = f::store(values);
        assert_eq!(
            text(Kind::Translation, &s, Some(4), &fallback),
            None,
            "case {case}"
        );
    }
    let s = f::store(attachments("source", "translated"));
    assert_eq!(text(Kind::Translation, &s, None, &fallback), None);
    assert_eq!(text(Kind::Translation, &s, Some(5), &fallback), None);
    assert_eq!(text(Kind::Ocr, &s, Some(5), &fallback), Some("source"));
}
#[test]
fn exact_plugin_type_schema_and_first_attachment_match_main_without_copying_foreign_payloads() {
    for (field, replacement) in [
        ("pluginId", "foreign"),
        ("typeId", "foreign.result"),
        ("schemaVersion", "2"),
    ] {
        let mut values = attachments("foreign text", "translated");
        values[0][field] = replacement.into();
        let s = f::store(values);
        assert_eq!(text(Kind::Ocr, &s, Some(4), &Legacy::default()), None);
    }
    let fallback = legacy(json!({"fullText":"old"}), Value::Null);
    for full in [Value::Null, json!(2), json!(" \n\t")] {
        let mut values = attachments("new", "translated");
        values[0]["payload"]["fullText"] = full;
        values[0].as_object_mut().unwrap().remove("payloadDigest");
        assert_eq!(text(Kind::Ocr, &f::store(values), Some(4), &fallback), None);
    }
}
#[test]
fn legacy_results_keep_whitespace_and_require_exact_translation_source_revision() {
    let l = legacy(
        json!({"fullText":" 原文\n "}),
        json!({"fullText":" 译文😀 ","sourceRevision":4}),
    );
    assert_eq!(
        text(Kind::Ocr, &Store::default(), None, &l),
        Some(" 原文\n ")
    );
    assert_eq!(
        text(Kind::Translation, &Store::default(), Some(4), &l),
        Some(" 译文😀 ")
    );
    for revision in [None, Some(0), Some(5)] {
        assert_eq!(
            text(Kind::Translation, &Store::default(), revision, &l),
            None
        );
    }
    for invalid in [
        Value::Null,
        json!(false),
        json!({"fullText":7}),
        json!({"fullText":" \n"}),
    ] {
        let l = legacy(invalid.clone(), invalid);
        assert_eq!(text(Kind::Ocr, &Store::default(), Some(0), &l), None);
        assert_eq!(
            text(Kind::Translation, &Store::default(), Some(0), &l),
            None
        );
    }
}
