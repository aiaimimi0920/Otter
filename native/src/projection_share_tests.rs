//! 分享编解码及 canonical source 归属；使用隔离原身份/存储夹具，不接触真实设备。
use super::*;
use crate::projection_delivery::sender::{
    self,
    binding::Binding,
    tests::{prepare, setup},
};
use base64::Engine;
use serde_json::{json, Value};
const ORIGIN: &str = "https://loom.example";
#[test]
fn public_journal_cannot_silently_reuse_targeted_receipt_identity() {
    let (dir, _store) = setup();
    let journal = crate::projection_delivery::journal::Store::open(&dir.0).unwrap();
    let public = crate::projection_delivery::accept::tests::public_invitation(ORIGIN);
    journal.begin(&public, "receiver-1").unwrap();
    let targeted = crate::projection_delivery::accept::tests::invitation(ORIGIN);
    assert!(journal.begin(&targeted, "receiver-1").is_err());
    assert!(!journal.pending().unwrap()[0]
        .invitation()
        .requires_receipt());
}
fn public_response(intent: &sender::Intent, receiver: bool) -> sender::Response {
    let value = json!({"envelope":intent.envelope(),"revision":1,"digest":intent.digest(),"linked":true,
        "delivery":null,"receiverDeviceId":receiver.then_some("receiver-1"),"receiverUnitId":receiver.then_some("received-unit")});
    sender::Response::parse(
        &serde_json::to_vec(&value).unwrap(),
        intent,
        false,
        &AtomicBool::new(false),
    )
    .unwrap()
}
#[test]
fn share_intent_persists_without_fake_target_and_primary_enters_canonical_index() {
    let (dir, store) = setup();
    let scope = "a".repeat(64);
    let choice = sender::Selection::invitation(&scope, ORIGIN, "unit-1").unwrap();
    let intent = prepare(&store, &dir, &choice).unwrap();
    assert_eq!(intent.target(), None);
    let disk: Value = serde_json::from_slice(&std::fs::read(dir.path()).unwrap()).unwrap();
    assert!(disk["entries"][0]["selection"].get("target").is_none());
    let source: crate::session_source::Source = serde_json::to_vec(&json!({"stickers":[{"id":"unit-1","x":0,"y":0,"w":4,"h":4,"future":{"keep":true}}],"links":[],"otterNativeProjectionScope":scope})).unwrap().into();
    let candidate = Binding::prepare(
        &source,
        &intent,
        &public_response(&intent, false),
        "分享邀请",
        &AtomicBool::new(false),
    )
    .unwrap();
    let value: Value = serde_json::from_slice(candidate.source()).unwrap();
    assert!(value["stickers"][0].get("projectionSenders").is_none());
    assert!(
        value["stickers"][0]["qrProjection"]["otterNativeSend"]["invitation"]
            .as_bool()
            .unwrap()
    );
    assert!(candidate.binding().is_invitation());
    assert!(!candidate.binding().linked());
    assert!(candidate.index().for_intent(&intent).is_some());
    let mut conflicting = value.clone();
    let mut second = conflicting["stickers"][0].clone();
    second["id"] = "unit-2".into();
    second["qrProjection"]["localUnitId"] = "unit-2".into();
    second["qrProjection"]["envelope"]["source"]["unitId"] = "unit-2".into();
    conflicting["stickers"].as_array_mut().unwrap().push(second);
    let conflicting: crate::session_source::Source =
        serde_json::to_vec(&conflicting).unwrap().into();
    assert!(sender::binding::checkpoint::Index::capture(&conflicting).is_none());
    let next = Binding::prepare(
        candidate.source(),
        &intent,
        &public_response(&intent, true),
        "分享邀请",
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(next.binding().linked());
    assert_eq!(next.binding().target(), None);
    let pending = sender::binding::stop::prepare(
        next.source(),
        next.binding(),
        false,
        &AtomicBool::new(false),
    )
    .unwrap();
    let index = sender::binding::checkpoint::Index::capture(&pending).unwrap();
    assert!(index.entries()[0].stopped());
    let removed = sender::binding::stop::prepare(
        &pending,
        &index.entries()[0],
        true,
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(sender::binding::checkpoint::Index::capture(&removed)
        .unwrap()
        .entries()
        .is_empty());
    let original = intent.envelope().clone();
    drop(next);
    drop(candidate);
    drop(intent);
    drop(store);
    let reopened = sender::Store::open(&dir.0, &AtomicBool::new(false)).unwrap();
    let restored = reopened.pending(&AtomicBool::new(false)).unwrap();
    assert_eq!(restored[0].envelope(), &original);
    assert_eq!(restored[0].target(), None);
}
#[test]
fn inspect_binds_current_digest_origin_and_public_receipt_identity() {
    use base64::engine::general_purpose::STANDARD;
    let (dir, store) = setup();
    let intent = prepare(
        &store,
        &dir,
        &sender::Selection::invitation(&"a".repeat(64), ORIGIN, "unit-1").unwrap(),
    )
    .unwrap();
    let bytes = crate::projection_delivery::accept::tests::png();
    let mut wire = json!({"envelope":intent.envelope(),"revision":3,"digest":intent.digest(),"linked":true,"width":2,"height":1,
        "receiverDeviceId":null,"receiverUnitId":null,"delivery":null,"snapshot":{"width":2,"height":1,"imageBase64":STANDARD.encode(&bytes)}});
    let parse = |value: &Value| {
        inspected(
            &serde_json::to_vec(value).unwrap(),
            intent.envelope(),
            ORIGIN,
            "receiver-1",
            &AtomicBool::new(false),
        )
    };
    let invitation = parse(&wire).unwrap();
    assert_eq!(invitation.revision(), 3);
    assert!(!invitation.requires_receipt());
    let body: Value = serde_json::from_slice(&invitation.accept_body(ORIGIN).unwrap()).unwrap();
    assert_eq!(body["expectedRevision"], 3);
    assert_eq!(body["expectedDigest"], intent.digest());
    assert_eq!(body["receiverUnitId"], invitation.receiver_unit_id());
    assert_eq!(body["confirmed"], true);
    for (field, value) in [
        ("width", json!(3)),
        ("height", json!(2)),
        (
            "snapshot",
            json!({"width":3,"height":1,"imageBase64":STANDARD.encode(&bytes)}),
        ),
        (
            "snapshot",
            json!({"width":2,"height":1,"imageBase64":STANDARD.encode(&bytes[..24])}),
        ),
        (
            "snapshot",
            json!({"width":2,"height":1,"imageBase64":"invalid!"}),
        ),
    ] {
        let mut invalid = wire.clone();
        invalid[field] = value;
        assert!(matches!(parse(&invalid), Err(Error::Snapshot)), "{field}");
    }
    assert!(inspected(
        &serde_json::to_vec(&wire).unwrap(),
        intent.envelope(),
        ORIGIN,
        "receiver-1",
        &AtomicBool::new(true),
    )
    .is_err());
    wire["digest"] = "b".repeat(64).into();
    assert!(parse(&wire).is_err());
    wire["digest"] = intent.digest().into();
    wire["receiverDeviceId"] = "other".into();
    wire["receiverUnitId"] = invitation.receiver_unit_id().into();
    assert!(parse(&wire).is_err());
    wire["receiverDeviceId"] = Value::Null;
    wire["receiverUnitId"] = Value::Null;
    wire["delivery"] = json!({"targetDeviceId":"other","status":"awaiting_confirmation"});
    assert!(parse(&wire).is_err());
}

#[test]
fn imported_share_alias_persists_authenticated_transport_and_peer_scope() {
    use base64::engine::general_purpose::STANDARD;
    let (dir, store) = setup();
    let intent = prepare(
        &store,
        &dir,
        &sender::Selection::invitation(&"a".repeat(64), ORIGIN, "unit-1").unwrap(),
    )
    .unwrap();
    let source = format!("loom-{}", "a".repeat(64));
    let transport = "https://192.168.15.20:8766";
    let mut wire = json!({"sourceLoomId":source,"route":"shared_loom","envelope":intent.envelope(),"revision":1,
        "digest":intent.digest(),"linked":true,"width":2,"height":1,"receiverDeviceId":null,"receiverUnitId":null,"delivery":null,
        "snapshot":{"width":2,"height":1,"imageBase64":STANDARD.encode(crate::projection_delivery::accept::tests::png())}});
    let import = |wire: &Value| {
        inspected_from(
            &serde_json::to_vec(wire).unwrap(),
            intent.envelope(),
            transport,
            "receiver-1",
            Some(&source),
            &AtomicBool::new(false),
        )
    };
    let invitation = import(&wire).unwrap();
    assert_eq!(invitation.origin(), transport);
    assert_eq!(invitation.envelope().origin(), ORIGIN);
    assert!(invitation.accept_body(transport).is_ok());
    assert!(invitation.accept_body("https://other.invalid").is_err());
    let journal = crate::projection_delivery::journal::Store::open(&dir.0).unwrap();
    journal.begin(&invitation, "receiver-1").unwrap();
    drop(journal);
    let journal = crate::projection_delivery::journal::Store::open(&dir.0).unwrap();
    assert_eq!(journal.pending().unwrap()[0].origin(), transport);
    let different_transport = inspected_from(
        &serde_json::to_vec(&wire).unwrap(),
        intent.envelope(),
        "https://192.168.15.21:8766",
        "receiver-1",
        Some(&source),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert!(matches!(
        journal.begin(&different_transport, "receiver-1"),
        Err(crate::projection_delivery::journal::Error::Changed)
    ));
    wire["sourceLoomId"] = format!("loom-{}", "b".repeat(64)).into();
    assert!(import(&wire).is_err());
    wire["sourceLoomId"] = source.clone().into();
    wire["route"] = "offline_peer".into();
    wire["peerId"] = source.clone().into();
    wire["delivery"] = json!({"targetDeviceId":"receiver-1","status":"awaiting_confirmation"});
    let peer = import(&wire).unwrap();
    assert_eq!(peer.origin(), transport);
    assert!(peer.requires_receipt());
    assert!(peer.accept_body(transport).is_ok());
}
