//! 旧字段合同、按库隔离和预算回归；不访问剪贴板、用户文件或系统时钟。
use super::*;
#[test]
fn standalone_reader_rejects_future_schema_and_byte_budget_before_parsing() {
    assert!(decode(
        br#"{"documentSchemaVersion":2,"referenceLibrary":[]}"#,
        Kind::ReferenceLibrary
    )
    .is_err());
    assert!(decode(
        &vec![b' '; crate::session_document::MAX_SESSION_BYTES + 1],
        Kind::RecycleBin
    )
    .is_err());
}
fn entry(id: &str) -> serde_json::Value {
    serde_json::json!({"entryId":id,"sourceStickerId":"source-1","createdAt":"2026-10-23T00:00:00Z",
        "snapshot":{"id":"source-1","src":"data:image/png;base64,fixture","x":-20,"y":30,"w":80,"h":60,
        "opacityNormal":0.7,"annotationState":{"future":true},"extensionState":{"unknown":[1,2]},"futureSnapshot":{"keep":true}},
        "futureEntry":{"keep":true}})
}
fn session(entries: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"stickers":[],"links":[],"recycleBin":entries})).unwrap()
}
#[test]
fn document_reads_frozen_records_without_dropping_unknown_fields_or_loading_sources() {
    let bytes = session(serde_json::json!([entry("entry-1")]));
    let document = crate::session_document::decode(&bytes).unwrap();
    let library = document.library(Kind::RecycleBin).unwrap();
    let entry = library.find("entry-1").unwrap();
    assert_eq!(library.entries().len(), 1);
    assert_eq!(entry.window.rect().x, -20.0);
    assert_eq!(entry.window.opacity(false), 0.7);
    assert!(entry.original_json().contains("futureEntry"));
    assert!(entry.snapshot_json().contains("extensionState"));
    assert!(entry.snapshot_json().contains("futureSnapshot"));
    let range = bytes.as_ptr() as usize..bytes.as_ptr() as usize + bytes.len();
    assert!(range.contains(&(entry.original_json().as_ptr() as usize)));
    assert!(range.contains(&(entry.snapshot_json().as_ptr() as usize)));
    assert_eq!(document.original_json(), bytes);
    assert!(document
        .library(Kind::ReferenceLibrary)
        .unwrap()
        .entries()
        .is_empty());
}
#[test]
fn invalid_library_does_not_block_active_windows_or_the_other_library() {
    let bytes = br#"{"stickers":[],"links":[],"recycleBin":42,"referenceLibrary":[]}"#;
    let document = crate::session_document::decode(bytes).unwrap();
    assert!(document.library(Kind::RecycleBin).is_err());
    assert!(document
        .library(Kind::ReferenceLibrary)
        .unwrap()
        .entries()
        .is_empty());
    assert!(decode(br#"{}"#, Kind::RecycleBin)
        .unwrap()
        .entries()
        .is_empty());
}
#[test]
fn duplicate_identity_invalid_geometry_and_active_art_are_rejected_atomically() {
    let good = entry("a");
    assert!(decode(
        &session(serde_json::json!([good.clone(), good.clone()])),
        Kind::RecycleBin
    )
    .is_err());
    for (field, value) in [
        ("w", serde_json::json!(0)),
        ("id", serde_json::json!("other")),
        ("type", serde_json::json!("art")),
        ("opacityNormal", serde_json::json!(2)),
    ] {
        let mut bad = good.clone();
        bad["snapshot"][field] = value;
        assert!(decode(
            &session(serde_json::json!([good.clone(), bad])),
            Kind::RecycleBin
        )
        .is_err());
    }
    let mut legacy = good;
    legacy["createdAt"] = serde_json::json!("legacy-invalid-time");
    assert!(decode(&session(serde_json::json!([legacy])), Kind::RecycleBin).is_ok());
}
#[test]
fn entry_limit_rejects_the_entire_library_instead_of_silently_pruning() {
    let entries: Vec<_> = (0..MAX_ENTRIES)
        .map(|n| entry(&format!("entry-{n}")))
        .collect();
    assert_eq!(
        decode(&session(serde_json::json!(entries)), Kind::RecycleBin)
            .unwrap()
            .entries()
            .len(),
        MAX_ENTRIES
    );
    let entries: Vec<_> = (0..=MAX_ENTRIES)
        .map(|n| entry(&format!("entry-{n}")))
        .collect();
    assert!(decode(&session(serde_json::json!(entries)), Kind::RecycleBin).is_err());
}
