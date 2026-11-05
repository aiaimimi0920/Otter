//! 真实旧合同形状与安全边界；每次全量拒绝，不发布部分目标或悬空组成员。
use super::*;
use serde_json::{json, Value};
const ORIGIN: &str = "https://loom.example";
pub(crate) fn body() -> Value {
    json!({"targets":[{"deviceId":"receiver-one","name":"A&B","policy":"confirm","route":"shared_loom"}],
        "groups":[{"groupId":"group:one","name":"Pair","targetIds":["receiver-one"],"unavailableCount":1}],
        "peerDirectory":{"status":"partial"}})
}
fn parse(value: &Value) -> Result<Directory, Error> {
    Directory::parse(&serde_json::to_vec(value).unwrap(), ORIGIN)
}
fn offline(available: bool) -> Value {
    use sha2::{Digest, Sha256};
    let peer = format!("loom-{}", "a".repeat(64));
    json!({"deviceId":format!("peer-target:{:x}",Sha256::digest(format!("{peer}\nremote:one"))),
        "name":"Remote", "policy":"auto","route":"offline_peer","peerId":peer,
        "peerName":"Peer","remoteDeviceId":"remote:one","deliveryAvailable":available,
        "transferProtocol":"loom.offline-transfer.v1","unavailableReason":"offline_peer_delivery_not_implemented"})
}
#[test]
fn projection_targets_keeps_typed_groups_origin_and_legacy_optional_fields() {
    let directory = parse(&body()).unwrap();
    assert_eq!(directory.origin(), ORIGIN);
    assert_eq!(directory.status(), Status::Partial);
    assert_eq!(directory.targets()[0].policy(), Policy::Confirm);
    assert_eq!(directory.targets()[0].name(), "A&B");
    assert!(matches!(directory.targets()[0].route(), Route::SharedLoom));
    let groups = directory.groups().unwrap();
    assert_eq!(groups[0].id(), "group:one");
    assert_eq!(groups[0].target_ids(), &["receiver-one"]);
    assert_eq!(groups[0].unavailable_count(), 1);
    let old = parse(&json!({"targets":[]})).unwrap();
    assert_eq!(old.status(), Status::Complete);
    assert!(old.groups().is_none());
    for status in ["complete", "partial", "busy", "unavailable"] {
        let mut value = body();
        value["peerDirectory"]["status"] = status.into();
        assert!(parse(&value).is_ok());
    }
}
#[test]
fn projection_targets_offline_hash_binding_and_advertised_availability_are_preserved() {
    for available in [false, true] {
        let value = json!({"targets":[offline(available)]});
        let directory = parse(&value).unwrap();
        assert!(
            matches!(directory.targets()[0].route(), Route::OfflinePeer {
            advertised_available, peer_id, remote_device_id, peer_name
        } if *advertised_available == available && peer_id.starts_with("loom-")
            && remote_device_id == "remote:one" && peer_name == "Peer")
        );
    }
}
#[test]
fn projection_targets_invalid_routes_and_peer_bindings_fail_whole_directory() {
    for (key, invalid) in [
        ("deviceId", json!("peer-target:bad")),
        ("peerId", json!(format!("loom-{}", "b".repeat(64)))),
        ("remoteDeviceId", json!("remote:other")),
        ("peerName", json!("")),
        ("peerName", json!("x\n")),
        ("peerName", json!("界".repeat(43))),
        ("route", json!("https://other.invalid")),
        ("deliveryAvailable", json!("true")),
        ("transferProtocol", json!("other")),
    ] {
        let mut target = offline(true);
        target[key] = invalid;
        assert!(
            parse(&json!({"targets":[body()["targets"][0].clone(),target]})).is_err(),
            "{key}"
        );
    }
    let mut unavailable = offline(false);
    unavailable["unavailableReason"] = "other".into();
    assert!(parse(&json!({"targets":[unavailable]})).is_err());
    for peer in [Value::Null, json!("loom-x")] {
        let mut value = body();
        value["targets"][0]["peerId"] = peer;
        assert!(parse(&value).is_err());
    }
}
#[test]
fn projection_targets_rejects_duplicate_ids_bad_policies_names_and_required_fields() {
    for (key, invalid) in [
        ("deviceId", json!("")),
        ("deviceId", json!("has space")),
        ("deviceId", json!("x".repeat(161))),
        ("deviceId", json!("peer-target:one")),
        ("name", json!("x\r\nforged")),
        ("name", json!("界".repeat(342))),
        ("policy", json!("disabled")),
        ("policy", json!(null)),
    ] {
        let mut value = body();
        value["targets"][0][key] = invalid;
        assert!(parse(&value).is_err(), "{key}");
    }
    for key in ["deviceId", "name", "policy", "route"] {
        let mut value = body();
        value["targets"][0].as_object_mut().unwrap().remove(key);
        assert!(parse(&value).is_err(), "{key}");
    }
    let mut value = body();
    let duplicate = value["targets"][0].clone();
    value["targets"].as_array_mut().unwrap().push(duplicate);
    assert!(matches!(parse(&value), Err(Error::Duplicate)));
}
#[test]
fn projection_targets_groups_reject_stale_duplicate_overflow_and_unbounded_membership() {
    for (key, invalid) in [
        ("groupId", json!("")),
        ("name", json!("")),
        ("name", json!("x\t")),
        ("name", json!("界".repeat(43))),
        ("targetIds", json!(["absent"])),
        ("targetIds", json!(["receiver-one", "receiver-one"])),
        ("unavailableCount", json!(-1)),
        ("unavailableCount", json!(0.5)),
        ("unavailableCount", json!(64)),
        ("unavailableCount", json!(u64::MAX)),
    ] {
        let mut value = body();
        value["groups"][0][key] = invalid;
        assert!(parse(&value).is_err(), "{key}");
    }
    let mut value = body();
    let group = value["groups"][0].clone();
    value["groups"].as_array_mut().unwrap().push(group);
    assert!(matches!(parse(&value), Err(Error::Duplicate)));
    let mut value = body();
    value["groups"][0]["targetIds"] = json!(vec!["receiver-one"; 65]);
    assert!(parse(&value).is_err());
}
#[test]
fn projection_targets_directory_and_wire_limits_fail_closed() {
    for (key, invalid) in [
        ("peerDirectory", Value::Null),
        ("groups", Value::Null),
        ("peerDirectory", json!({})),
        ("peerDirectory", json!({"status":"unknown"})),
        ("targets", json!({})),
        ("groups", json!({})),
    ] {
        let mut value = body();
        value[key] = invalid;
        assert!(parse(&value).is_err(), "{key}");
    }
    let mut value = body();
    value["targets"] = json!(vec![value["targets"][0].clone(); 65]);
    assert!(matches!(parse(&value), Err(Error::Limit)));
    let mut value = body();
    value["groups"] = json!(vec![value["groups"][0].clone(); 33]);
    assert!(matches!(parse(&value), Err(Error::Limit)));
    assert!(matches!(
        Directory::parse(&vec![b' '; MAX_JSON + 1], ORIGIN),
        Err(Error::Limit)
    ));
    assert!(Directory::parse(br#"{"targets":[]}"#, "http://loom.example").is_err());
    assert!(Directory::parse(b"{", ORIGIN).is_err());
}
