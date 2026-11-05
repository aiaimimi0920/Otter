use super::*;
use serde_json::{json, Value};
#[cfg(windows)]
#[path = "projection_receive_offline_tests.rs"]
pub(crate) mod offline;
pub(crate) fn invitation(origin: &str, accepted: bool) -> Value {
    json!({"envelope":{"protocol":"neuro.qr-projection.v1",
        "projectionId":format!("projection:{}", "1".repeat(32)),"serverOrigin":origin,
        "source":{"deviceId":"source-1","sessionId":"session-1","unitId":"unit-1","revision":3},
        "content":{"kind":"sticker","digest":"a".repeat(64)},
        "expiresAtMs":envelope::MAX_REVISION,"nonce":"2".repeat(32),
        "signature":{"algorithm":"ed25519","keyId":"source-1","value":"s".repeat(86)}},
        "revision":4,"digest":"b".repeat(64),"sourceName":"测试设备",
        "delivery":{"status":if accepted {"accepted"} else {"awaiting_confirmation"}},
        "receivePolicy":"confirm"})
}
pub(crate) fn body(origin: &str) -> Vec<u8> {
    encode(vec![invitation(origin, false)])
}
fn encode(entries: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"invitations":entries})).unwrap()
}
const ORIGIN: &str = "https://loom.example";
#[test]
fn shared_inbox_preserves_identity_versions_and_accepted_expired_resume_candidates() {
    let mut accepted = invitation(ORIGIN, true);
    accepted["envelope"]["projectionId"] = format!("projection:{}", "3".repeat(32)).into();
    accepted["envelope"]["expiresAtMs"] = 1.into();
    accepted.as_object_mut().unwrap().remove("sourceName");
    let bytes = serde_json::to_vec(
        &json!({"invitations":[invitation(ORIGIN,false),accepted],"offlinePeers":true}),
    )
    .unwrap();
    let inbox = Inbox::parse(&bytes, ORIGIN).unwrap();
    assert!(inbox.has_offline_peers());
    let first = &inbox.entries()[0];
    assert_eq!(first.envelope().origin(), ORIGIN);
    assert_eq!(first.envelope().source_revision(), 3);
    assert_eq!(first.source_name(), "测试设备");
    assert_eq!(first.revision(), 4);
    assert_eq!(first.digest(), "b".repeat(64));
    assert!(!first.accepted());
    assert!(inbox.entries()[1].accepted());
    assert_eq!(inbox.entries()[1].source_name(), "source-1");
    let debug = format!("{:?}", first.envelope());
    assert!(!debug.contains(&"s".repeat(86)) && !debug.contains(&"2".repeat(32)));
}
#[test]
fn whole_inbox_rejects_ambiguous_duplicate_and_cross_origin_entries() {
    let good = invitation(ORIGIN, false);
    assert!(matches!(
        Inbox::parse(&encode(vec![good.clone(), good.clone()]), ORIGIN),
        Err(Error::Duplicate)
    ));
    let bad = invitation("https://other.example", false);
    assert!(matches!(
        Inbox::parse(&encode(vec![good, bad]), ORIGIN),
        Err(Error::Origin)
    ));
    assert!(Inbox::parse(&body(ORIGIN), "http://loom.example").is_err());
}
#[test]
fn envelope_shape_protocol_canonical_origin_signature_and_identifier_fail_closed() {
    for (key, value) in [
        ("protocol", json!("neuro.qr-projection.v2")),
        ("projectionId", json!("projection:../x")),
        ("serverOrigin", json!("https://loom.example/")),
        ("nonce", json!("z".repeat(32))),
        ("expiresAtMs", json!(0)),
        ("expiresAtMs", json!(envelope::MAX_REVISION + 1)),
    ] {
        let mut bad = invitation(ORIGIN, false);
        bad["envelope"][key] = value;
        assert!(Inbox::parse(&encode(vec![bad]), ORIGIN).is_err(), "{key}");
    }
    for (section, key, value) in [
        ("source", "revision", json!(true)),
        ("source", "revision", json!(0)),
        ("source", "unitId", json!("unit\r\nforged")),
        ("content", "digest", json!("A".repeat(64))),
        ("content", "kind", json!("surface")),
        ("signature", "algorithm", json!("none")),
        ("signature", "keyId", json!("other-device")),
        ("signature", "value", json!("x".repeat(87))),
    ] {
        let mut bad = invitation(ORIGIN, false);
        bad["envelope"][section][key] = value;
        assert!(
            Inbox::parse(&encode(vec![bad]), ORIGIN).is_err(),
            "{section}.{key}"
        );
    }
    let mut bad = invitation(ORIGIN, false);
    bad["envelope"]["credential"] = "must-not-be-accepted".into();
    assert!(Inbox::parse(&encode(vec![bad]), ORIGIN).is_err());
}
#[test]
fn invitation_revision_route_delivery_and_name_limits_are_atomic() {
    for (key, value) in [
        ("revision", json!(2)),
        ("revision", json!(envelope::MAX_REVISION + 1)),
        ("digest", json!("A".repeat(64))),
        ("sourceName", json!("x".repeat(1025))),
        ("sourceName", json!("fake\nmenu")),
        ("sourceName", json!("")),
        ("receivePolicy", json!("disabled")),
        ("route", json!("offline_peer")),
        ("delivery", json!({"status":"displayed"})),
    ] {
        let mut bad = invitation(ORIGIN, false);
        bad["envelope"]["projectionId"] = format!("projection:{}", "3".repeat(32)).into();
        bad[key] = value;
        assert!(
            matches!(
                Inbox::parse(&encode(vec![invitation(ORIGIN, false), bad]), ORIGIN),
                Err(Error::Entry)
            ),
            "{key}"
        );
    }
}
#[test]
fn entry_byte_and_duplicate_field_limits_do_not_silently_truncate() {
    let entries: Vec<_> = (0..64)
        .map(|id| {
            let mut item = invitation(ORIGIN, false);
            item["envelope"]["projectionId"] = format!("projection:{id:032x}").into();
            item
        })
        .collect();
    assert_eq!(
        Inbox::parse(&encode(entries.clone()), ORIGIN)
            .unwrap()
            .entries()
            .len(),
        64
    );
    let mut too_many = entries;
    too_many.push(invitation(ORIGIN, false));
    assert!(matches!(
        Inbox::parse(&encode(too_many), ORIGIN),
        Err(Error::Limit)
    ));
    assert!(matches!(
        Inbox::parse(&vec![b' '; MAX_JSON + 1], ORIGIN),
        Err(Error::Limit)
    ));
    assert!(Inbox::parse(br#"{"invitations":[],"invitations":[]}"#, ORIGIN).is_err());
    let data = String::from_utf8(body(ORIGIN)).unwrap();
    assert!(Inbox::parse(
        data.replace("\"revision\":4", "\"revision\":4,\"revision\":5")
            .as_bytes(),
        ORIGIN
    )
    .is_err());
}
#[test]
fn rejection_is_only_for_same_origin_new_invitation_even_after_invitation_expiry() {
    let mut expired = invitation(ORIGIN, false);
    expired["envelope"]["expiresAtMs"] = 1.into();
    let inbox = Inbox::parse(&encode(vec![expired]), ORIGIN).unwrap();
    let item = &inbox.entries()[0];
    let body: Value = serde_json::from_slice(&item.reject_body(ORIGIN).unwrap()).unwrap();
    assert_eq!(
        body,
        json!({"projectionId":item.envelope().projection_id(),"status":"rejected"})
    );
    assert_eq!(
        item.reject_body("https://other.example"),
        Err(Error::Origin)
    );
    let accepted = Inbox::parse(&encode(vec![invitation(ORIGIN, true)]), ORIGIN).unwrap();
    assert_eq!(
        accepted.entries()[0].reject_body(ORIGIN),
        Err(Error::AlreadyAccepted)
    );
}
#[test]
fn receipt_needs_typed_recorded_true_and_never_accepts_a_displayed_claim() {
    assert_eq!(receipt(br#"{"recorded":true}"#), Ok(()));
    for bad in [
        br#"{"recorded":false}"#.as_slice(),
        br#"{"recorded":"true"}"#,
        br#"{"status":"displayed"}"#,
        br#"{"recorded":false,"recorded":true}"#,
    ] {
        assert!(receipt(bad).is_err());
    }
}
