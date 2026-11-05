//! 保留缓冲区的拓扑裁剪、库消费和失败不变性；无需 HWND 或用户文件。
use super::*;

#[test]
fn unrelated_or_tampered_window_cannot_consume_selected_entry() {
    let original = source();
    let text = String::from_utf8(copy()).unwrap();
    for bad in [
        text.replace("true", "false"),
        text.replace("\"w\":10", "\"w\":20"),
        text.replace("\"params\":{}", "\"params\":{\"execute\":true}"),
        text.replace("\"params\":{}", "\"params\":{},\"outputs\":{}"),
    ] {
        assert!(prepare(
            &original,
            bad.as_bytes(),
            &ids(&["a"]),
            Kind::RecycleBin,
            "entry",
            &AtomicBool::new(false)
        )
        .is_err());
    }
    assert_eq!(original, source());
}

fn source() -> Vec<u8> {
    br#"{"documentRevision":9,"stickers":[{"id":"a","x":0,"y":0,"w":10,"h":10},{"id":"closed","x":0,"y":0,"w":10,"h":10}],
    "links":[{"id":"edge","fromUnitId":"a","fromPortId":"out","toUnitId":"closed","toPortId":"in"}],
    "recycleBin":[{"entryId":"entry","sourceStickerId":"frozen","createdAt":"legacy","snapshot":{"id":"frozen","x":0,"y":0,"w":10,"h":10,"extension": { "keep" : true }}}],
    "rootExtension": { "huge":123456789012345678901234567890 }}"#.to_vec()
}
fn copy() -> Vec<u8> {
    br#"{"stickers":[{"id":"fresh","type":"sticker","params":{},"x":50,"y":50,"w":10,"h":10,"extension": { "keep" : true }}],"links":[]}"#.to_vec()
}
fn ids(values: &[&str]) -> Vec<UnitId> {
    values
        .iter()
        .map(|value| UnitId::parse(value).unwrap())
        .collect()
}

#[test]
fn recycle_retention_preserves_raw_extensions_and_prunes_only_closed_records() {
    let original = source();
    let bytes = prepare(
        &original,
        &copy(),
        &ids(&["a"]),
        Kind::RecycleBin,
        "entry",
        &AtomicBool::new(false),
    )
    .unwrap();
    let document = session_document::decode(bytes.as_ref()).unwrap();
    assert_eq!(document.revision, 9);
    assert_eq!(
        document
            .windows
            .iter()
            .map(|window| window.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "fresh"]
    );
    assert!(document.workspace.links().is_empty());
    assert!(document
        .library(Kind::RecycleBin)
        .unwrap()
        .entries()
        .is_empty());
    let raw = std::str::from_utf8(bytes.as_ref()).unwrap();
    assert!(raw.contains(r#"{ "huge":123456789012345678901234567890 }"#));
    assert!(raw.contains(r#"{ "keep" : true }"#));
    assert_eq!(original, source());
}

#[test]
fn reference_retention_keeps_library_and_existing_edges() {
    let source = String::from_utf8(source())
        .unwrap()
        .replace("recycleBin", "referenceLibrary");
    let bytes = prepare(
        source.as_bytes(),
        &copy(),
        &ids(&["a", "closed"]),
        Kind::ReferenceLibrary,
        "entry",
        &AtomicBool::new(false),
    )
    .unwrap();
    let document = session_document::decode(bytes.as_ref()).unwrap();
    assert_eq!(document.workspace.links().len(), 1);
    assert_eq!(
        document
            .library(Kind::ReferenceLibrary)
            .unwrap()
            .entries()
            .len(),
        1
    );
}

#[test]
fn invalid_identity_capacity_entry_and_cancel_return_no_retention() {
    let original = source();
    let copy = copy();
    for (active, entry, cancel) in [
        (ids(&["fresh"]), "entry", false),
        (ids(&["a"]), "missing", false),
        (ids(&["a"]), "entry", true),
        (
            (0..crate::workspace::MAX_UNITS)
                .map(|index| UnitId::parse(&format!("unit-{index}")).unwrap())
                .collect(),
            "entry",
            false,
        ),
    ] {
        assert!(prepare(
            &original,
            &copy,
            &active,
            Kind::RecycleBin,
            entry,
            &AtomicBool::new(cancel)
        )
        .is_err());
    }
    for name in ["a", "frozen"] {
        let bad = String::from_utf8(copy.clone())
            .unwrap()
            .replace("fresh", name);
        assert!(prepare(
            &original,
            bad.as_bytes(),
            &ids(&["a"]),
            Kind::RecycleBin,
            "entry",
            &AtomicBool::new(false)
        )
        .is_err());
    }
    assert_eq!(original, source());
}
