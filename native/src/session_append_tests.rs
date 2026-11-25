//! Single-sticker original-text identity/capacity/cancellation/unknown-field protection, without remote authorization/library-entry consumption.
use super::*;
use serde_json::{json, Value};
fn prepare(
    original: &[u8],
    instance: &[u8],
    active: &[UnitId],
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    let source = decode_source(original)?;
    let instance = session_document::decode(instance)
        .map_err(|error| format!("Invalid instance: {error:?}"))?;
    Prepared::new(&source, &instance, active)?.finish(None, cancel)
}
fn unit(id: &str) -> Value {
    json!({"id":id,"type":"sticker","x":0,"y":0,"w":100,"h":80,"params":{}})
}
fn source() -> Vec<u8> {
    serde_json::to_vec(&json!({"documentRevision":7,"stickers":[unit("a"),unit("b"),unit("closed")],
        "links":[{"id":"keep","fromUnitId":"a","fromPortId":"output","toUnitId":"b","toPortId":"image"},
        {"id":"stale","fromUnitId":"a","fromPortId":"output","toUnitId":"closed","toPortId":"image"}],
        "rootExtension":{"keep":"原文"}})).unwrap()
}
fn instance() -> Vec<u8> {
    let mut sticker = unit("delivery-11111111111111111111111111111111");
    sticker["qrProjection"] = json!({"role":"receiver","revision":5,"future":{"keep":true}});
    serde_json::to_vec(&json!({"stickers":[sticker],"links":[]})).unwrap()
}
#[test]
fn append_preserves_unknown_fields_and_filters_only_closed_stickers_and_links_without_revision_bump(
) {
    let bytes = prepare(
        &source(),
        &instance(),
        &[UnitId::parse("a").unwrap(), UnitId::parse("b").unwrap()],
        &AtomicBool::new(false),
    )
    .unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["documentRevision"], 7);
    assert_eq!(value["rootExtension"]["keep"], "原文");
    assert_eq!(value["stickers"].as_array().unwrap().len(), 3);
    assert_eq!(value["links"].as_array().unwrap().len(), 1);
    assert_eq!(value["links"][0]["id"], "keep");
    assert_eq!(value["stickers"][2]["qrProjection"]["future"]["keep"], true);
    assert!(!value["stickers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["id"] == "closed"));
}
#[test]
fn append_cannot_reuse_active_or_retained_identity_consume_capacity_or_ignore_cancel() {
    let new = UnitId::parse("delivery-11111111111111111111111111111111").unwrap();
    assert!(prepare(
        &source(),
        &instance(),
        std::slice::from_ref(&new),
        &AtomicBool::new(false)
    )
    .is_err());
    let mut copy: Value = serde_json::from_slice(&instance()).unwrap();
    copy["stickers"][0]["id"] = "closed".into();
    assert!(prepare(
        &source(),
        &serde_json::to_vec(&copy).unwrap(),
        &[],
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(prepare(
        &source(),
        &instance(),
        &vec![UnitId::parse("a").unwrap(); crate::workspace::MAX_UNITS],
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(prepare(&source(), &instance(), &[], &AtomicBool::new(true)).is_err());
}
#[test]
fn malformed_duplicate_root_and_missing_arrays_never_panic_or_partially_publish() {
    let source = source();
    let text = std::str::from_utf8(&source).unwrap().replacen(
        "\"documentRevision\":7",
        "\"documentRevision\":7,\"documentRevision\":7",
        1,
    );
    assert!(prepare(text.as_bytes(), &instance(), &[], &AtomicBool::new(false)).is_err());
    for invalid in [
        br#"{"stickers":[]}"#.as_slice(),
        br#"{"links":[]}"#,
        br#"{}"#,
        br#"[]"#,
    ] {
        assert!(prepare(invalid, &instance(), &[], &AtomicBool::new(false)).is_err());
    }
    assert!(prepare(
        &source,
        br#"{"stickers":[],"links":[]}"#,
        &[],
        &AtomicBool::new(false)
    )
    .is_err());
}
#[test]
fn decoded_instance_cannot_skip_strict_raw_field_checks() {
    let bytes = br#"{"stickers":[{"id":"new","type":"sticker","x":0,"y":0,"w":1,"h":1,"params":{}}],"links":[],"future":1,"future":2}"#;
    let original = source();
    let source = decode_source(&original).unwrap();
    let instance = session_document::decode(bytes).unwrap();
    let error = Prepared::new(&source, &instance, &[]).err().unwrap();
    assert!(error.contains("Session field limit or duplicate"));
}
