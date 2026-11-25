//! 删除单条不会变成取消同源全部收藏，也不消费另一库或未知原文。
use super::*;
fn source() -> Vec<u8> {
    let entries: Vec<_> = (0..3).map(|i| serde_json::json!({"entryId":format!("entry-{i}"),"sourceStickerId":"shared-source",
        "createdAt":"legacy", "extension":{"kept":i},"snapshot":{"id":"shared-source","x":0,"y":0,"w":32,"h":32}})).collect();
    serde_json::to_vec(&serde_json::json!({"documentRevision":7,"stickers":[],"links":[],"recycleBin":entries,"referenceLibrary":entries})).unwrap()
}
#[test]
fn removes_exact_identity_in_selected_library_without_advancing_revision() {
    for kind in [Kind::RecycleBin, Kind::ReferenceLibrary] {
        let original = source();
        let candidate = prepare(&original, kind, "entry-1", &AtomicBool::new(false)).unwrap();
        assert_eq!((candidate.expected_revision, candidate.revision), (7, 7));
        let bytes = candidate.bytes;
        let selected = super::super::decode(&bytes, kind).unwrap();
        assert_eq!(selected.entries().len(), 2);
        assert!(selected.find("entry-1").is_none());
        let before = super::super::decode(&original, kind).unwrap();
        assert_eq!(
            selected.find("entry-2").unwrap().original_json(),
            before.find("entry-2").unwrap().original_json()
        );
        let other = match kind {
            Kind::RecycleBin => Kind::ReferenceLibrary,
            Kind::ReferenceLibrary => Kind::RecycleBin,
        };
        assert_eq!(
            super::super::decode(&bytes, other).unwrap().entries().len(),
            3
        );
        assert_eq!(crate::session_document::decode(&bytes).unwrap().revision, 7);
        assert!(prepare(&bytes, kind, "entry-1", &AtomicBool::new(false)).is_err());
    }
}
#[test]
fn rejects_missing_invalid_or_cancelled_identity_and_preserves_unrelated_raw_values() {
    let original = source();
    for id in ["", "missing"] {
        assert!(prepare(&original, Kind::RecycleBin, id, &AtomicBool::new(false)).is_err());
    }
    assert!(prepare(
        &original,
        Kind::RecycleBin,
        "entry-1",
        &AtomicBool::new(true)
    )
    .is_err());
    let text = String::from_utf8(original).unwrap();
    let text = format!(
        "{{\"precise\":123456789012345678901234567890,{}",
        &text[1..]
    );
    let bytes = prepare(
        text.as_bytes(),
        Kind::ReferenceLibrary,
        "entry-1",
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(std::str::from_utf8(&bytes.bytes)
        .unwrap()
        .contains("123456789012345678901234567890"));
    assert!(prepare(
        br#"{"stickers":[],"links":[],"referenceLibrary":42}"#,
        Kind::ReferenceLibrary,
        "entry-1",
        &AtomicBool::new(false)
    )
    .is_err());
}
#[test]
fn clear_only_selected_library_preserves_revision_windows_and_unknown_fields() {
    let original = serde_json::to_vec(&serde_json::json!({"documentRevision":7,"stickers":[],"links":[],
        "recycleBin":[{"entryId":"r","sourceStickerId":"pin","createdAt":"legacy","snapshot":{"id":"pin","x":0,"y":0,"w":32,"h":32}}],
        "referenceLibrary":[{"entryId":"f","sourceStickerId":"pin","createdAt":"legacy","snapshot":{"id":"pin","x":0,"y":0,"w":32,"h":32}}],
        "unknown":{"nested":[1,2,3]}})).unwrap();
    for kind in [Kind::RecycleBin, Kind::ReferenceLibrary] {
        let candidate = super::clear(&original, kind, &AtomicBool::new(false))
            .unwrap()
            .unwrap();
        assert_eq!((candidate.expected_revision, candidate.revision), (7, 7));
        let bytes = candidate.bytes;
        assert!(super::clear(&bytes, kind, &AtomicBool::new(false))
            .unwrap()
            .is_none());
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let other = match kind {
            Kind::RecycleBin => Kind::ReferenceLibrary,
            Kind::ReferenceLibrary => Kind::RecycleBin,
        };
        assert!(super::super::decode(&bytes, kind)
            .unwrap()
            .entries()
            .is_empty());
        assert_eq!(
            super::super::decode(&bytes, other).unwrap().entries().len(),
            1
        );
        assert_eq!(value["documentRevision"], 7);
        assert_eq!(value["unknown"], serde_json::json!({"nested":[1,2,3]}));
        assert_eq!(value["stickers"], serde_json::json!([]));
        assert!(super::clear(&original, kind, &AtomicBool::new(true)).is_err());
    }
}
#[test]
fn clear_rejects_corrupt_selected_library_and_does_not_materialize_missing_one() {
    assert!(super::clear(
        br#"{"stickers":[],"links":[],"recycleBin":42}"#,
        Kind::RecycleBin,
        &AtomicBool::new(false)
    )
    .is_err());
    let original = b"  {\"stickers\":[],\"links\":[],\"other\":123}\n";
    assert!(
        super::clear(original, Kind::RecycleBin, &AtomicBool::new(false))
            .unwrap()
            .is_none()
    );
}
