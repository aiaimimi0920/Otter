//! 显式接收路由跨写前日志/工作区绑定保存恢复；缺失兼容不等于允许null降级。
use super::*;
use crate::projection_delivery::{
    binding::Binding,
    journal::{tests::Directory, Store},
};
#[test]
fn offline_receiver_journal_reopen_keeps_transport_peer_and_original_intent() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let item = invitation(ORIGIN, false);
    let ticket = store.begin(&item, "device-1").unwrap();
    let id = ticket.intent_id().to_owned();
    let bytes = std::fs::read(directory.path()).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value["entries"][0]["offline"],
        json!({"origin":ORIGIN,"peerId":PEER})
    );
    assert_eq!(value["entries"][0]["envelope"]["serverOrigin"], SOURCE);
    for other in [
        invitation("https://other.example", false),
        accept::tests::invitation(SOURCE),
    ] {
        assert_eq!(
            store.begin(&other, "device-1").err(),
            Some(journal::Error::Changed)
        );
    }
    let mut other = item.clone();
    other.offline.as_mut().unwrap().peer_id = format!("loom-{}", "c".repeat(64));
    assert_eq!(
        store.begin(&other, "device-1").err(),
        Some(journal::Error::Changed)
    );
    assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    drop((ticket, store));
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.pending().unwrap().pop().unwrap();
    assert_eq!(ticket.intent_id(), id);
    assert_eq!(ticket.origin(), ORIGIN);
    assert_eq!(ticket.invitation().peer_id(), Some(PEER));
    assert_eq!(ticket.envelope(), item.envelope());
    assert_eq!(
        ticket.path(ReceiveOperation::Read),
        "/v1/offline-projections/read"
    );
    assert_eq!(
        displayed_body(&ticket, SOURCE, "device-1"),
        Err(Error::Origin)
    );
    assert!(displayed_body(&ticket, ORIGIN, "device-1").is_ok());
}
#[test]
fn offline_receiver_corrupt_persisted_route_fails_closed_without_replacing_journal() {
    let original = {
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        store.begin(&invitation(ORIGIN, false), "device-1").unwrap();
        serde_json::from_slice::<Value>(&std::fs::read(directory.path()).unwrap()).unwrap()
    };
    for route in [
        Value::Null,
        json!({"origin":ORIGIN}),
        json!({"origin":"http://local.example","peerId":PEER}),
        json!({"origin":ORIGIN,"peerId":"bad"}),
        json!({"origin":ORIGIN,"peerId":PEER,"token":"never"}),
    ] {
        let mut value = original.clone();
        value["entries"][0]["offline"] = route;
        let directory = Directory::new();
        let bytes = serde_json::to_vec(&value).unwrap();
        std::fs::write(directory.path(), &bytes).unwrap();
        assert!(Store::open(&directory.0).is_err());
        assert_eq!(std::fs::read(directory.path()).unwrap(), bytes);
    }
}
#[test]
fn offline_receiver_binding_matches_both_persisted_route_parts_and_rejects_partial_markers() {
    let directory = Directory::new();
    let store = Store::open(&directory.0).unwrap();
    let ticket = store.begin(&invitation(ORIGIN, false), "device-1").unwrap();
    let image = verified(ORIGIN, "device-1").bind_intent(&ticket).unwrap();
    let binding = Binding::new(&image, &ticket).unwrap();
    let value = serde_json::to_value(&binding).unwrap();
    assert_eq!(value["offlineTransport"], json!({"origin":ORIGIN}));
    assert_eq!(value["otterNativeReceive"]["peerId"], PEER);
    assert_eq!(value["envelope"]["serverOrigin"], SOURCE);
    let restored: Binding = serde_json::from_value(value.clone()).unwrap();
    assert!(restored.matches_ticket(&ticket));
    for (part, key) in [(false, "offlineTransport"), (true, "peerId")] {
        for null in [false, true] {
            let mut bad = value.clone();
            let object = if part {
                &mut bad["otterNativeReceive"]
            } else {
                &mut bad
            };
            if null {
                object[key] = Value::Null;
            } else {
                object.as_object_mut().unwrap().remove(key);
            }
            assert!(
                !serde_json::from_value::<Binding>(bad).is_ok_and(|v| v.matches_ticket(&ticket))
            );
        }
    }
    for (part, key, text) in [
        (false, "origin", "https://other.example"),
        (
            true,
            "peerId",
            "loom-cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        ),
    ] {
        let mut bad = value.clone();
        if part {
            bad["otterNativeReceive"][key] = text.into();
        } else {
            bad["offlineTransport"][key] = text.into();
        }
        assert!(!serde_json::from_value::<Binding>(bad)
            .unwrap()
            .matches_ticket(&ticket));
    }
    let online_directory = Directory::new();
    let online_store = Store::open(&online_directory.0).unwrap();
    let online = online_store
        .begin(&accept::tests::invitation(SOURCE), "device-1")
        .unwrap();
    assert!(image.bind_intent(&online).is_err());
}
