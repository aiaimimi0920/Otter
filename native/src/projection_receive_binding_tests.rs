//! 关联严格匹配意图/设备/完整快照；旧会话仍可保存，但不能凭 Unit 或未知元数据推断包含。
use super::*;
use crate::projection_delivery::{
    accept::tests as png,
    journal::{tests::Directory, Store},
};
use serde_json::{json, Value};
const ORIGIN: &str = "https://loom.example";
fn fixture() -> (Directory, Ticket, Binding) {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.begin(&png::invitation(ORIGIN), "device-1").unwrap();
    let binding = Binding::new(
        &png::verified(ORIGIN).bind_intent(&ticket).unwrap(),
        &ticket,
    )
    .unwrap();
    (directory, ticket, binding)
}
fn session(binding: &Binding) -> Value {
    json!({"documentSchemaVersion":1,"documentRevision":1,"stickers":[{
        "id":binding.local_unit_id,"type":"sticker","x":0,"y":0,"w":2,"h":1,
        "qrProjection":binding}],"links":[],"groups":[]})
}
fn source(value: &Value) -> crate::session_source::Source {
    serde_json::to_vec(value).unwrap().into()
}
#[test]
fn binding_requires_exact_intent_envelope_and_verified_receiver_device() {
    let (directory, ticket, binding) = fixture();
    assert!(binding.matches_ticket(&ticket));
    assert_eq!(binding.revision, 5);
    assert_eq!(ticket.invitation().revision(), 4);
    let other = Directory::new();
    let other_store = Store::open(&other.0).unwrap();
    let same_projection = other_store
        .begin(&png::invitation(ORIGIN), "device-1")
        .unwrap();
    assert!(!binding.matches_ticket(&same_projection));
    let old_verified = png::verified(ORIGIN).bind_intent(&ticket).unwrap();
    assert!(Binding::new(&old_verified, &same_projection).is_err());
    assert!(old_verified.bind_intent(&same_projection).is_err());
    assert!(Binding::new(&png::verified(ORIGIN), &ticket).is_err());
    let wrong = Directory::new();
    let wrong_store = Store::open(&wrong.0).unwrap();
    let wrong_ticket = wrong_store
        .begin(&png::invitation(ORIGIN), "device-other")
        .unwrap();
    assert!(png::verified(ORIGIN).bind_intent(&wrong_ticket).is_err());
    assert!(Binding::new(&png::verified(ORIGIN), &wrong_ticket).is_err());
    let origin_ticket = wrong_store
        .begin(&png::invitation("https://other.example"), "device-1")
        .unwrap();
    assert!(png::verified(ORIGIN).bind_intent(&origin_ticket).is_err());
    assert!(Binding::new(&png::verified(ORIGIN), &origin_ticket).is_err());
    assert!(std::fs::read(directory.path())
        .unwrap()
        .windows(ticket.intent_id().len())
        .any(|x| x == ticket.intent_id().as_bytes()));
}
#[test]
fn index_is_bound_to_exact_shared_source_and_full_expected_binding() {
    let (_directory, ticket, binding) = fixture();
    let bytes = source(&session(&binding));
    let index = checkpoint::Index::capture(&bytes).unwrap();
    assert!(index.is_for(&bytes.clone()));
    let equal_bytes: crate::session_source::Source = bytes.to_vec().into();
    assert!(!index.is_for(&equal_bytes));
    assert!(index.includes(&binding));
    assert!(index.for_ticket(&ticket).is_some());
    for (field, replacement) in [("revision", json!(6)), ("digest", json!("c".repeat(64)))] {
        let mut changed = session(&binding);
        changed["stickers"][0]["qrProjection"][field] = replacement;
        let index = checkpoint::Index::capture(&source(&changed)).unwrap();
        assert!(!index.includes(&binding), "{field}");
    }
}
#[test]
fn malformed_or_ambiguous_native_binding_never_proves_inclusion_without_breaking_session() {
    let (_directory, _ticket, binding) = fixture();
    for (field, replacement) in [
        ("role", json!("source")),
        ("localUnitId", json!("other")),
        ("linked", json!(false)),
        ("revision", json!(0)),
        ("digest", json!("invalid")),
        (
            "otterNativeReceive",
            json!({"intentId":"not-a-guid","deviceId":"device-1"}),
        ),
    ] {
        let mut changed = session(&binding);
        changed["stickers"][0]["qrProjection"][field] = replacement;
        let bytes = source(&changed);
        assert!(crate::session_document::decode(&bytes).is_ok());
        assert!(checkpoint::Index::capture(&bytes).is_none(), "{field}");
    }
    let text = serde_json::to_string(&session(&binding)).unwrap();
    for duplicate in ["\"role\":\"receiver\",", "\"otterNativeReceive\":{},"] {
        let text = text.replace(
            "\"qrProjection\":{",
            &format!("\"qrProjection\":{{{duplicate}"),
        );
        assert!(checkpoint::Index::capture(&text.into_bytes().into()).is_none());
    }
}
#[test]
fn legacy_projection_and_oversized_metadata_are_preserved_but_not_assumed_native() {
    let (_directory, ticket, binding) = fixture();
    let mut mixed = session(&binding);
    mixed["stickers"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"ordinary","x":0,"y":0,"w":1,"h":1,"qrProjection":null}));
    let index = checkpoint::Index::capture(&source(&mixed)).unwrap();
    assert!(index.includes(&binding));
    assert!(index.for_ticket(&ticket).is_some());
    let mut legacy = session(&binding);
    legacy["stickers"][0]["qrProjection"]
        .as_object_mut()
        .unwrap()
        .remove("otterNativeReceive");
    let index = checkpoint::Index::capture(&source(&legacy)).unwrap();
    assert!(!index.includes(&binding));
    assert!(index.for_ticket(&ticket).is_none());
    let mut large = session(&binding);
    large["stickers"][0]["qrProjection"]["unknown"] = "x".repeat(8192).into();
    let bytes = source(&large);
    assert!(crate::session_document::decode(&bytes).is_ok());
    assert!(checkpoint::Index::capture(&bytes).is_none());
}
#[test]
fn index_rejects_duplicate_intents_and_obeys_existing_workspace_capacity() {
    let (_directory, _ticket, binding) = fixture();
    let mut value = session(&binding);
    for index in 1..=crate::workspace::MAX_UNITS {
        let mut sticker = value["stickers"][0].clone();
        let id = format!("delivery-{index:032x}");
        sticker["id"] = id.clone().into();
        sticker["qrProjection"]["localUnitId"] = id.into();
        sticker["qrProjection"]["envelope"]["projectionId"] =
            format!("projection:{index:032x}").into();
        if index == 1 {
            value["stickers"]
                .as_array_mut()
                .unwrap()
                .push(sticker.clone());
            assert!(checkpoint::Index::capture(&source(&value)).is_none());
            value["stickers"].as_array_mut().unwrap().pop();
        }
        sticker["qrProjection"]["otterNativeReceive"]["intentId"] =
            format!("00000000-0000-0000-0000-{index:012x}").into();
        value["stickers"].as_array_mut().unwrap().push(sticker);
        assert_eq!(
            checkpoint::Index::capture(&source(&value)).is_some(),
            index < crate::workspace::MAX_UNITS
        );
    }
}
