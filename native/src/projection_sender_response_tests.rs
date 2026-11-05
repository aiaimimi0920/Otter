//! 发送响应不能复用 receiver 的非空字段假设；完整原身份与内容仍须严格相关。
use super::*;
use crate::projection_delivery::sender::tests::{prepare, selection, setup};
use serde_json::{json, Value};
fn response(intent: &Intent) -> Value {
    use base64::{engine::general_purpose::STANDARD, Engine};
    json!({"envelope":intent.envelope(),"revision":1,"digest":intent.digest(),"linked":true,
        "receiverDeviceId":null,"receiverUnitId":null,"delivery":{"targetDeviceId":intent.target(),"status":"awaiting_confirmation"},
        "snapshot":{"imageBase64":STANDARD.encode(crate::projection_delivery::accept::tests::png()),"width":2,"height":1}})
}
fn parse(value: &Value, intent: &Intent, create: bool) -> Result<Response, Error> {
    Response::parse(
        &serde_json::to_vec(value).unwrap(),
        intent,
        create,
        &AtomicBool::new(false),
    )
}
#[test]
fn awaiting_and_resumed_source_replies_keep_nullable_receiver_and_monotonic_revision() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let mut value = response(&intent);
    let first = parse(&value, &intent, true).unwrap();
    assert_eq!(first.status(), Status::AwaitingConfirmation);
    assert_eq!(first.receiver_device(), None);
    value["revision"] = 3.into();
    value["digest"] = "b".repeat(64).into();
    value["snapshot"] = Value::Null;
    value["receiverDeviceId"] = intent.target().into();
    value["receiverUnitId"] = "receiver-unit".into();
    value["delivery"]["status"] = "displayed".into();
    let resumed = parse(&value, &intent, false).unwrap();
    assert_eq!(resumed.revision(), 3);
    assert_eq!(resumed.receiver_unit(), Some("receiver-unit"));
    assert_eq!(resumed.digest(), "b".repeat(64));
    assert!(parse(&value, &intent, true).is_err());
    assert!(!format!("{resumed:?}").contains("receiver-unit"));
}
#[test]
fn wrong_identity_target_link_revision_digest_and_receiver_pairs_are_rejected() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let good = response(&intent);
    for (path, value) in [
        ("/envelope/nonce", json!("3".repeat(32))),
        ("/delivery/targetDeviceId", json!("other")),
        ("/linked", json!(false)),
        ("/revision", json!(0)),
        (
            "/revision",
            json!(super::super::super::envelope::MAX_REVISION + 1),
        ),
        ("/digest", json!("b".repeat(64))),
        ("/receiverDeviceId", json!(intent.target())),
        ("/receiverUnitId", json!("unit-only")),
        ("/delivery/status", json!("rejected")),
        ("/delivery/status", json!("unknown")),
        ("/delivery/status", json!("accepted")),
        ("/delivery/status", json!("displayed")),
    ] {
        let mut bad = good.clone();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(parse(&bad, &intent, false).is_err(), "{path}");
    }
    let mut wrong = good;
    wrong["receiverDeviceId"] = "other".into();
    wrong["receiverUnitId"] = "unit".into();
    assert!(parse(&wrong, &intent, false).is_err());
    wrong["receiverDeviceId"] = intent.target().into();
    assert!(parse(&wrong, &intent, false).is_err());
    wrong["delivery"]["status"] = "accepted".into();
    assert!(parse(&wrong, &intent, false).is_ok());
}
#[test]
fn nonempty_editing_and_bad_png_are_not_silently_downgraded_to_source_metadata() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    for mutation in [
        json!({"basis":{"digest":intent.digest()}}),
        json!(false),
        json!([]),
    ] {
        let mut value = response(&intent);
        value["editing"] = mutation;
        assert!(parse(&value, &intent, false).is_err());
    }
    let mut value = response(&intent);
    value["snapshot"]["width"] = 3.into();
    assert!(parse(&value, &intent, false).is_err());
    value = response(&intent);
    value["snapshot"]["imageBase64"] = "not-base64".into();
    assert!(parse(&value, &intent, false).is_err());
    value = response(&intent);
    value["editing"] = Value::Null;
    assert!(parse(&value, &intent, false).is_ok());
}
#[test]
fn valid_editing_is_bound_to_response_digest_and_actual_snapshot_dimensions() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let mut value = response(&intent);
    let mut document = crate::projection_edit::fixture::document_value();
    document["basis"] = json!({"digest":intent.digest(),"width":2,"height":1});
    value["editing"] = document;
    assert_eq!(
        parse(&value, &intent, false)
            .unwrap()
            .editing()
            .unwrap()
            .basis()
            .digest(),
        intent.digest()
    );
    value["editing"]["basis"]["digest"] = json!("b".repeat(64));
    assert!(parse(&value, &intent, false).is_err());
    value["editing"]["basis"]["digest"] = json!(intent.digest());
    value["editing"]["basis"]["width"] = json!(3);
    assert!(parse(&value, &intent, false).is_err());
}
#[test]
fn size_duplicate_fields_and_cancellation_fail_before_yielding_source_metadata() {
    let (directory, store) = setup();
    let intent = prepare(&store, &directory, &selection(1)).unwrap();
    let bytes = serde_json::to_vec(&response(&intent)).unwrap();
    assert_eq!(
        Response::parse(&bytes, &intent, false, &AtomicBool::new(true)).err(),
        Some(Error::Stopped)
    );
    assert_eq!(
        Response::parse(
            &vec![0; super::super::super::accept::MAX_RESPONSE + 1],
            &intent,
            false,
            &AtomicBool::new(false)
        )
        .err(),
        Some(Error::Limit)
    );
    let mut duplicate = bytes;
    duplicate.pop();
    duplicate.extend_from_slice(br#", "linked":true}"#);
    assert!(Response::parse(&duplicate, &intent, false, &AtomicBool::new(false)).is_err());
}
