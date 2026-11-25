//! Current catalog contracts/security boundaries; reject entire batches without partial targets/dangling group members.
use super::*;
use serde_json::{json, Value};
const ORIGIN: &str = "https://loom.example";
pub(crate) fn body() -> Value {
    json!({"targets":[{"deviceId":"receiver-one","name":"A&B","policy":"confirm","route":"shared_loom"}],
        "groups":[{"groupId":"group:one","name":"Pair","targetIds":["receiver-one"],"unavailableCount":1}],
        "peerDirectory":{"status":"partial"},"friends":{"users":[]}})
}

#[test]
fn logged_in_users_preserve_account_scope_and_reject_dangling_targets() {
    let mut value = body();
    value["friends"] = json!({"users":[{"userId":"user:one","issuer":"https://account.example",
        "accountId":"account-1","name":"登录用户","targetIds":["receiver-one"]}]});
    let directory = parse(&value).unwrap();
    assert_eq!(directory.users()[0].name(), "登录用户");
    assert_eq!(directory.users()[0].target_ids(), &["receiver-one"]);
    for (key, invalid) in [
        ("targetIds", json!(["missing"])),
        ("targetIds", json!(["receiver-one", "receiver-one"])),
        ("issuer", json!("http://external.invalid")),
        ("accountId", json!("")),
        ("name", json!("bad\nname")),
    ] {
        let mut bad = value.clone();
        bad["friends"]["users"][0][key] = invalid;
        assert!(parse(&bad).is_err(), "{key}");
    }
    let duplicate = value["friends"]["users"][0].clone();
    value["friends"]["users"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(parse(&value).is_err());
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
fn projection_targets_requires_current_directory_fields_and_keeps_origin() {
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
    for field in ["peerDirectory", "groups", "friends"] {
        let mut incomplete = body();
        incomplete.as_object_mut().unwrap().remove(field);
        assert!(parse(&incomplete).is_err());
    }
    for status in ["complete", "partial", "busy", "unavailable"] {
        let mut value = body();
        value["peerDirectory"]["status"] = status.into();
        assert!(parse(&value).is_ok());
    }
}
#[test]
fn projection_targets_offline_hash_binding_and_advertised_availability_are_preserved() {
    for available in [false, true] {
        let value = json!({"targets":[offline(available)],"groups":[],"peerDirectory":{"status":"complete"},"friends":{"users":[]}});
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
    let mut baseline = body();
    baseline["targets"]
        .as_array_mut()
        .unwrap()
        .push(offline(true));
    assert_eq!(parse(&baseline).unwrap().targets().len(), 2);
    for (key, invalid, expected) in [
        ("deviceId", json!("peer-target:bad"), Error::Entry),
        (
            "peerId",
            json!(format!("loom-{}", "b".repeat(64))),
            Error::Entry,
        ),
        ("remoteDeviceId", json!("remote:other"), Error::Entry),
        ("peerName", json!(""), Error::Entry),
        ("peerName", json!("x\n"), Error::Entry),
        ("peerName", json!("界".repeat(43)), Error::Entry),
        ("route", json!("https://other.invalid"), Error::Json),
        ("deliveryAvailable", json!("true"), Error::Json),
        ("transferProtocol", json!("other"), Error::Entry),
    ] {
        let mut value = baseline.clone();
        value["targets"][1][key] = invalid;
        assert_eq!(parse(&value).unwrap_err(), expected, "{key}");
    }
    baseline["targets"][1] = offline(false);
    assert_eq!(parse(&baseline).unwrap().targets().len(), 2);
    baseline["targets"][1]["unavailableReason"] = "other".into();
    assert_eq!(parse(&baseline).unwrap_err(), Error::Entry);
    for peer in [Value::Null, json!("loom-x")] {
        let mut value = body();
        value["targets"][0]["peerId"] = peer;
        assert_eq!(parse(&value).unwrap_err(), Error::Entry);
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
