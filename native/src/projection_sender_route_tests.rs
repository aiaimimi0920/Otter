//! 路由/写前日志/兼容binding闭环；oracle独立构造复合ID和响应，畸形路由不授在线资格。
use super::super::tests::{prepare, setup};
use super::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
const ORIGIN: &str = "https://loom.example";
fn selection() -> Selection {
    let peer = format!("loom-{}", "b".repeat(64));
    let id = format!(
        "peer-target:{:x}",
        Sha256::digest(format!("{peer}\nremote-one"))
    );
    Selection::offline(&"a".repeat(64), ORIGIN, "unit-1", &id, &peer, "remote-one").unwrap()
}
fn response(intent: &Intent) -> Value {
    json!({"envelope":intent.envelope(),"revision":1,"digest":intent.digest(),"linked":true,
        "receiverDeviceId":"remote-one","receiverUnitId":"remote-unit",
        "route":"offline_peer","peerId":format!("loom-{}", "b".repeat(64)),
        "delivery":{"targetDeviceId":intent.target(),"status":"displayed"}})
}
fn parse(value: &Value, intent: &Intent) -> Result<Response, Error> {
    Response::parse(
        &serde_json::to_vec(value).unwrap(),
        intent,
        false,
        &AtomicBool::new(false),
    )
}
#[test]
fn offline_route_roundtrips_original_signed_intent_and_cancel_without_shared_downgrade() {
    let (directory, store) = setup();
    let selected = selection();
    let intent = prepare(&store, &directory, &selected).unwrap();
    let envelope = intent.envelope().clone();
    assert_eq!(envelope.origin(), ORIGIN);
    assert_eq!(
        intent.path(Operation::Create),
        "/v1/offline-projections/create"
    );
    drop(intent);
    drop(store);
    let store = Store::open(&directory.0, &AtomicBool::new(false)).unwrap();
    let intent = store.pending(&AtomicBool::new(false)).unwrap().remove(0);
    assert_eq!(intent.envelope(), &envelope);
    assert!(intent.record.selection == selected);
    store.cancel(&intent).unwrap();
    assert_eq!(
        prepare(&store, &directory, &selected).err(),
        Some(Error::Cancelled)
    );
    let bytes = std::fs::read(directory.path()).unwrap();
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    for route in [
        Value::Null,
        json!({"peerId":"wrong","remoteDeviceId":"remote-one"}),
        json!({"peerId":"x".repeat(2048)}),
    ] {
        value["entries"][0]["selection"]["offline"] = route;
        assert!(record::Document::decode(
            &serde_json::to_vec(&value).unwrap(),
            &AtomicBool::new(false)
        )
        .is_err());
    }
    value["entries"][0]["selection"]
        .as_object_mut()
        .unwrap()
        .remove("offline");
    assert!(record::Document::decode(
        &serde_json::to_vec(&value).unwrap(),
        &AtomicBool::new(false)
    )
    .is_err());
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
}
#[test]
fn offline_response_requires_exact_peer_route_remote_receiver_and_never_accepts_transport_origin() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection()).unwrap();
    assert!(parse(&response(&intent), &intent).is_ok());
    for (field, bad) in [
        ("route", json!("shared_loom")),
        ("peerId", json!(format!("loom-{}", "c".repeat(64)))),
        ("peerId", Value::Null),
        ("receiverDeviceId", json!(intent.target())),
        (
            "offlineTransport",
            json!({"origin":"https://attacker.example"}),
        ),
        ("offlineTransport", Value::Null),
    ] {
        let mut value = response(&intent);
        value[field] = bad;
        assert!(parse(&value, &intent).is_err(), "{field}");
    }
    let mut value = response(&intent);
    value.as_object_mut().unwrap().remove("route");
    assert!(parse(&value, &intent).is_err());
    let shared = prepare(&store, &directory, &super::super::tests::selection(1)).unwrap();
    assert!(parse(&response(&shared), &shared).is_err());
}
#[test]
fn offline_binding_preserves_web_identity_and_only_explicit_native_route_can_restore_or_admit() {
    let (directory, store) = setup();
    let selected = selection();
    let intent = prepare(&store, &directory, &selected).unwrap();
    let original: crate::session_source::Source = serde_json::to_vec(&json!({"stickers":[
        {"id":"unit-1","x":0,"y":0,"w":4,"h":4}],"links":[],"otterNativeProjectionScope":"a".repeat(64)})).unwrap().into();
    let response = parse(&response(&intent), &intent).unwrap();
    let candidate = binding::Binding::prepare(
        &original,
        &intent,
        &response,
        "Remote",
        &AtomicBool::new(false),
    )
    .unwrap();
    let saved: Value = serde_json::from_slice(candidate.source()).unwrap();
    let entry = &saved["stickers"][0]["projectionSenders"][0];
    assert_eq!(entry["target"]["deviceId"], "remote-one");
    assert_eq!(
        entry["target"]["peerId"],
        format!("loom-{}", "b".repeat(64))
    );
    assert_eq!(entry["link"]["offlineTransport"]["origin"], ORIGIN);
    assert_eq!(entry["link"]["otterNativeSend"]["target"], intent.target());
    assert!(candidate.binding().matches_intent(&intent));
    assert!(candidate.index().for_intent(&intent).is_some());
    assert!(binding::Binding::admit_send(candidate.source(), &selected).is_err());
    // 同名本机设备与跨Loom设备不是同一目的；跨peer同名设备也应保持独立。
    let shared = Selection::new(&"a".repeat(64), ORIGIN, "unit-1", "remote-one").unwrap();
    assert!(binding::Binding::admit_send(candidate.source(), &shared).is_ok());
    let again = binding::Binding::prepare(
        candidate.source(),
        &intent,
        &response,
        "Renamed",
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(again.index().entries().len(), 1);
    for field in ["origin", "peer", "marker"] {
        let mut value = saved.clone();
        let entry = &mut value["stickers"][0]["projectionSenders"][0];
        match field {
            "origin" => {
                entry["link"]["offlineTransport"]["origin"] = "https://attacker.example".into()
            }
            "peer" => entry["target"]["peerId"] = Value::Null,
            _ => {
                entry["link"]["otterNativeSend"]
                    .as_object_mut()
                    .unwrap()
                    .remove("target");
            }
        }
        let bytes: crate::session_source::Source = serde_json::to_vec(&value).unwrap().into();
        assert!(
            binding::checkpoint::Index::capture(&bytes).is_none(),
            "{field}"
        );
        assert!(crate::session_document::decode(&bytes).is_ok());
    }
}
