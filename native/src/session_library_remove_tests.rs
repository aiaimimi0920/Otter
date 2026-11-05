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
        let bytes = prepare(&original, kind, "entry-1", &AtomicBool::new(false)).unwrap();
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
    assert!(std::str::from_utf8(&bytes)
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
