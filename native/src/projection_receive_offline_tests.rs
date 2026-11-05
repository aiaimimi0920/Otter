//! OfflinePeer模型夹具固定source与peer；预期路由独立写出，不从被测路由生成。
use super::*;
pub(crate) const SOURCE: &str = "https://source-loom.example";
pub(crate) const PEER: &str =
    "loom-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
#[path = "projection_receive_offline_read_tests.rs"]
mod read;
#[path = "projection_receive_offline_state_tests.rs"]
mod state;
pub(crate) fn entry(accepted: bool) -> Value {
    let mut value = super::invitation(SOURCE, accepted);
    value["route"] = "offline_peer".into();
    value["peerId"] = PEER.into();
    value
}
pub(crate) fn body(accepted: bool) -> Vec<u8> {
    super::encode(vec![entry(accepted)])
}
pub(crate) fn invitation(origin: &str, accepted: bool) -> Invitation {
    Inbox::parse_offline(&body(accepted), origin)
        .unwrap()
        .into_entries()
        .pop()
        .unwrap()
}
pub(crate) fn response(device: &str) -> Value {
    let mut value = accept::tests::response(SOURCE);
    value["route"] = "offline_peer".into();
    value["peerId"] = PEER.into();
    value["receiverDeviceId"] = device.into();
    value["delivery"]["targetDeviceId"] = device.into();
    value
}
pub(crate) fn verified(origin: &str, device: &str) -> accept::Verified {
    let origin = origin.to_owned();
    let device = device.to_owned();
    std::thread::spawn(move || {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            )
            .ok()
            .unwrap();
        }
        let value = accept::Received::parse(
            &serde_json::to_vec(&response(&device)).unwrap(),
            &invitation(&origin, false),
            &origin,
            &device,
        )
        .unwrap()
        .verify(|| false)
        .unwrap();
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
        value
    })
    .join()
    .unwrap()
}
#[test]
fn offline_receiver_keeps_source_envelope_and_local_transport_separate_for_all_operations() {
    let item = invitation(ORIGIN, false);
    assert_eq!(item.origin(), ORIGIN);
    assert_eq!(item.envelope().origin(), SOURCE);
    assert_eq!(item.peer_id(), Some(PEER));
    for (operation, path) in [
        (ReceiveOperation::Accept, "/v1/offline-projections/accept"),
        (ReceiveOperation::Read, "/v1/offline-projections/read"),
        (ReceiveOperation::Receipt, "/v1/offline-projections/receipt"),
        (ReceiveOperation::Unlink, "/v1/offline-projections/unlink"),
    ] {
        assert_eq!(item.path(operation), path);
    }
    let value: Value = serde_json::from_slice(&item.accept_body(ORIGIN).unwrap()).unwrap();
    assert_eq!(value["envelope"], entry(false)["envelope"]);
    assert_eq!(value["expectedRevision"], 4);
    assert_eq!(value["expectedDigest"], "b".repeat(64));
    assert_eq!(
        value["receiverUnitId"],
        format!("delivery-{}", "1".repeat(32))
    );
    assert!(value.get("peerId").is_none());
    assert!(value.get("offlineTransport").is_none());
    assert_eq!(item.accept_body(SOURCE), Err(Error::Origin));
    assert!(item.reject_body(ORIGIN).is_ok());
    assert_eq!(
        invitation(ORIGIN, true).reject_body(ORIGIN),
        Err(Error::AlreadyAccepted)
    );
}
#[test]
fn offline_receiver_inbox_rejects_missing_null_or_untrusted_routing_without_downgrade() {
    for (key, value) in [
        ("route", json!("shared_loom")),
        ("route", Value::Null),
        ("peerId", json!("loom-short")),
        ("peerId", json!(format!("loom-{}", "B".repeat(64)))),
        ("peerId", Value::Null),
        ("offlineTransport", Value::Null),
        (
            "offlineTransport",
            json!({"origin":"https://attacker.example"}),
        ),
    ] {
        let mut bad = entry(false);
        bad[key] = value;
        assert!(
            Inbox::parse_offline(&super::encode(vec![bad]), ORIGIN).is_err(),
            "{key}"
        );
    }
    for key in ["route", "peerId"] {
        let mut bad = entry(false);
        bad.as_object_mut().unwrap().remove(key);
        assert!(Inbox::parse_offline(&super::encode(vec![bad]), ORIGIN).is_err());
    }
    assert!(Inbox::parse(&body(false), ORIGIN).is_err());
    let mut bad = entry(false);
    bad["envelope"]["serverOrigin"] = "http://source.example".into();
    assert!(Inbox::parse_offline(&super::encode(vec![bad]), ORIGIN).is_err());
    assert!(Inbox::parse_offline(&body(false), "https://local.example/").is_ok());
}
#[test]
fn offline_receiver_merge_is_atomic_on_cross_route_collision_and_combined_limit() {
    let shared = || {
        let mut shared = super::invitation(ORIGIN, false);
        shared["envelope"]["projectionId"] = format!("projection:{}", "2".repeat(32)).into();
        Inbox::parse(
            &serde_json::to_vec(&json!({"invitations":[shared],"offlinePeers":true})).unwrap(),
            ORIGIN,
        )
        .unwrap()
    };
    let mut inbox = shared();
    inbox
        .merge_offline(Inbox::parse_offline(&body(true), ORIGIN).unwrap())
        .unwrap();
    assert_eq!(inbox.entries().len(), 2);
    assert!(!inbox.has_offline_peers());
    assert_eq!(inbox.entries()[1].peer_id(), Some(PEER));
    let mut inbox = shared();
    let mut collision = entry(false);
    collision["envelope"]["projectionId"] = inbox.entries()[0].envelope().projection_id().into();
    assert_eq!(
        inbox.merge_offline(Inbox::parse_offline(&super::encode(vec![collision]), ORIGIN).unwrap()),
        Err(Error::Duplicate)
    );
    assert_eq!(inbox.entries().len(), 1);
    assert!(inbox.has_offline_peers());
    let entries = (0..64)
        .map(|i| {
            let mut v = entry(false);
            v["envelope"]["projectionId"] = format!("projection:{i:032x}").into();
            v
        })
        .collect();
    assert_eq!(
        inbox.merge_offline(Inbox::parse_offline(&super::encode(entries), ORIGIN).unwrap()),
        Err(Error::Limit)
    );
    assert_eq!(inbox.entries().len(), 1);
    assert!(inbox.has_offline_peers());
    inbox
        .merge_offline(Inbox::parse_offline(br#"{"invitations":[]}"#, ORIGIN).unwrap())
        .unwrap();
    assert_eq!(inbox.entries().len(), 1);
    assert!(!inbox.has_offline_peers());
}
