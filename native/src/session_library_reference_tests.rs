//! Same-source favorites replacement/removal, source-field fidelity, and failure atomicity, without HWNDs or disk.
use super::*;
use std::sync::atomic::AtomicBool;
const ORIGINAL: &[u8] = br#"{"documentSchemaVersion":1,"documentRevision":7,"stickers":[],"links":[],"extension":{"huge":123456789012345678901234567890},"recycleBin":42}"#;
const SNAPSHOT: &[u8] = br#"{"id":"sticker","type":"sticker","x":1,"y":2,"w":32,"h":40,"params":{},"inputs":[],"outputs":{},"portVisibility":{},"otterNativeProjectionEdit":{},"extensionState":{"precise":123456789012345678901234567890}}"#;
fn addition<'a>(
    snapshot: &'a [u8],
    entry_id: &'a str,
    created_at: &'a str,
) -> Option<Addition<'a>> {
    Some(Addition {
        snapshot,
        entry_id,
        created_at,
    })
}
fn unit() -> UnitId {
    UnitId::parse("sticker").unwrap()
}
#[test]
fn adding_library_cannot_create_an_unreadable_257_field_root() {
    let mut root = serde_json::Map::new();
    root.insert("stickers".into(), serde_json::json!([]));
    root.insert("links".into(), serde_json::json!([]));
    for i in 0..254 {
        root.insert(format!("extension-{i}"), serde_json::json!(i));
    }
    let original = serde_json::to_vec(&root).unwrap();
    assert!(prepare(
        &original,
        addition(SNAPSHOT, "entry", "date"),
        &unit(),
        &AtomicBool::new(false)
    )
    .is_err());
    root.remove("extension-253");
    let original = serde_json::to_vec(&root).unwrap();
    let result = set(&original, "entry");
    assert_eq!(
        candidate::fields(std::str::from_utf8(&result).unwrap())
            .unwrap()
            .len(),
        256
    );
    assert!(prepare(&result, None, &unit(), &AtomicBool::new(false)).is_ok());
}
fn set(source: &[u8], id: &str) -> Pixels {
    prepare(
        source,
        addition(SNAPSHOT, id, "2026-10-23T00:00:00.000Z"),
        &unit(),
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn freezes_current_snapshot_preserves_unknown_raw_and_does_not_advance_disk_revision() {
    let bytes = set(ORIGINAL, "entry-1");
    let document = session_document::decode(&bytes).unwrap();
    assert_eq!(document.revision, 7);
    assert!(document.windows.is_empty());
    let library = super::super::decode(&bytes, Kind::ReferenceLibrary).unwrap();
    let entry = &library.entries()[0];
    assert_eq!(entry.source_sticker_id, "sticker");
    let snapshot = candidate::fields(entry.snapshot_json()).unwrap();
    assert!(
        !snapshot.contains_key("params")
            && !snapshot.contains_key("inputs")
            && !snapshot.contains_key("outputs")
            && !snapshot.contains_key("portVisibility")
            && !snapshot.contains_key("otterNativeProjectionEdit")
    );
    assert_eq!(
        snapshot["extensionState"].get(),
        r#"{"precise":123456789012345678901234567890}"#
    );
    let root = candidate::fields(std::str::from_utf8(&bytes).unwrap()).unwrap();
    assert_eq!(
        root["extension"].get(),
        r#"{"huge":123456789012345678901234567890}"#
    );
    assert_eq!(root["recycleBin"].get(), "42"); // An unrelated corrupt library must neither block favorites nor be opportunistically repaired/cleared.
}
#[test]
fn same_source_replaces_and_remove_by_source_keeps_other_entries() {
    let first = set(ORIGINAL, "entry-1");
    let second = set(&first, "entry-2");
    let library = super::super::decode(&second, Kind::ReferenceLibrary).unwrap();
    assert_eq!(library.entries().len(), 1);
    assert_eq!(library.entries()[0].entry_id, "entry-2");
    assert!(super::super::decode(&first, Kind::ReferenceLibrary)
        .unwrap()
        .find("entry-1")
        .is_some());
    let other = UnitId::parse("other").unwrap();
    let unchanged = prepare(&second, None, &other, &AtomicBool::new(false)).unwrap();
    assert_eq!(
        super::super::decode(&unchanged, Kind::ReferenceLibrary)
            .unwrap()
            .entries()[0]
            .original_json(),
        library.entries()[0].original_json()
    );
    let removed = prepare(&second, None, &unit(), &AtomicBool::new(false)).unwrap();
    assert!(super::super::decode(&removed, Kind::ReferenceLibrary)
        .unwrap()
        .entries()
        .is_empty());
    assert_eq!(session_document::decode(&removed).unwrap().revision, 7);
}
#[test]
fn invalid_source_identity_art_cancel_and_duplicate_root_do_not_produce_candidates() {
    let false_flag = AtomicBool::new(false);
    assert!(prepare(
        ORIGINAL,
        addition(SNAPSHOT, "", "date"),
        &unit(),
        &false_flag
    )
    .is_err());
    assert!(prepare(
        ORIGINAL,
        addition(SNAPSHOT, "entry", "\n"),
        &unit(),
        &false_flag
    )
    .is_err());
    assert!(prepare(
        ORIGINAL,
        addition(SNAPSHOT, "entry", "date"),
        &UnitId::parse("missing").unwrap(),
        &false_flag
    )
    .is_err());
    assert!(prepare(
        ORIGINAL,
        addition(SNAPSHOT, "entry", "date"),
        &unit(),
        &AtomicBool::new(true)
    )
    .is_err());
    let art = String::from_utf8(SNAPSHOT.to_vec())
        .unwrap()
        .replace("\"type\":\"sticker\"", "\"type\":\"art\"");
    assert!(prepare(
        ORIGINAL,
        addition(art.as_bytes(), "entry", "date"),
        &unit(),
        &false_flag
    )
    .is_err());
    let first = set(ORIGINAL, "entry");
    assert!(prepare(
        &first,
        addition(SNAPSHOT, "entry", "date"),
        &unit(),
        &false_flag
    )
    .is_err());
    assert!(prepare(
        br#"{"stickers":[],"links":[],"extra":1,"extra":2}"#,
        addition(SNAPSHOT, "entry", "date"),
        &unit(),
        &false_flag
    )
    .is_err());
}
#[test]
fn full_library_can_replace_same_source_but_cannot_admit_an_extra_source() {
    let entries: Vec<_> = (0..MAX_ENTRIES).map(|i| serde_json::json!({"entryId":format!("entry-{i}"),"sourceStickerId":format!("source-{i}"),"createdAt":"legacy", "snapshot":{"id":format!("source-{i}"),"x":0,"y":0,"w":32,"h":32}})).collect();
    let source = serde_json::to_vec(
        &serde_json::json!({"stickers":[],"links":[],"referenceLibrary":entries}),
    )
    .unwrap();
    assert!(prepare(
        &source,
        addition(SNAPSHOT, "new-entry", "date"),
        &unit(),
        &AtomicBool::new(false)
    )
    .is_err());
    let replacement = String::from_utf8(SNAPSHOT.to_vec())
        .unwrap()
        .replace("\"sticker\"", "\"source-0\"")
        .replace("\"type\":\"source-0\"", "\"type\":\"sticker\"");
    let bytes = prepare(
        &source,
        addition(replacement.as_bytes(), "new-entry", "date"),
        &UnitId::parse("source-0").unwrap(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let library = super::super::decode(&bytes, Kind::ReferenceLibrary).unwrap();
    assert_eq!(library.entries().len(), MAX_ENTRIES);
    assert_eq!(library.entries().last().unwrap().entry_id, "new-entry");
}
