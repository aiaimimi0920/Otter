use super::*;
use serde_json::{json, Value};
fn observation(id: &str, sequence: u64) -> Value {
    json!({"observationId":id,"sequence":sequence,"state":"stable",
        "source":"app_adapter","confidence":"exact","observedAtMs":1,"value":42})
}
fn encoded(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
#[test]
fn validates_trust_locator_value_and_strict_fields() {
    let valid = observation("test", 1);
    assert!(Observation::parse(&encoded(&valid)).is_ok());
    for (key, value) in [
        ("sequence", json!(0)),
        ("observedAtMs", json!(0)),
        ("state", json!("stale")),
        ("state", json!("error")),
        ("state", json!("unknown")),
        ("source", json!("vision")),
        ("source", json!("unknown")),
        ("source", json!("ui_automation")),
        ("reason", json!("")),
        ("invented", json!(true)),
        ("value", json!("x".repeat(65536))),
    ] {
        let mut value_copy = valid.clone();
        value_copy[key] = value;
        assert!(Observation::parse(&encoded(&value_copy)).is_err(), "{key}");
    }
    let mut uia = valid;
    uia["source"] = json!("ui_automation");
    uia["locator"] = json!({"controlType":"Button","ancestorPath":[]});
    assert!(Observation::parse(&encoded(&uia)).is_ok());
    uia["locator"]["ancestorPath"] = json!(vec!["parent"; 65]);
    assert!(Observation::parse(&encoded(&uia)).is_err());
}
#[test]
fn snapshot_requires_field_and_rejects_duplicates_and_capacity_overflow() {
    assert!(Observations::from_snapshot(b"{}").is_err());
    assert!(Observations::from_snapshot(br#"{"observations":[]}"#).is_ok());
    let a = observation("a", 9);
    assert!(matches!(
        Observations::from_snapshot(&encoded(&json!({"observations":[a,a]}))),
        Err(Error::Duplicate)
    ));
    let values: Vec<_> = (0..257)
        .map(|i| observation(&format!("id:{i}"), 1))
        .collect();
    assert!(matches!(
        Observations::from_snapshot(&encoded(&json!({"observations":values}))),
        Err(Error::Limit)
    ));
}
#[test]
fn incremental_batch_is_atomic_and_sequence_exhaustion_never_replays() {
    let mut state =
        Observations::from_snapshot(&encoded(&json!({"observations":[observation("a",9)]})))
            .unwrap();
    let a10 = encoded(&observation("a", 10));
    let b2 = encoded(&observation("b", 2));
    assert_eq!(
        state.apply([a10.as_slice(), b2.as_slice()]),
        Err(Error::Sequence)
    );
    assert_eq!(state.get("a").unwrap().sequence, 9);
    assert!(state.get("b").is_none());
    let b1 = encoded(&observation("b", 1));
    state.apply([a10.as_slice(), b1.as_slice()]).unwrap();
    assert_eq!(state.iter().count(), 2);
    assert_eq!(state.get("a").unwrap().sequence, 10);
    assert_eq!(state.apply([a10.as_slice()]), Err(Error::Sequence));
    let mut exhausted = Observations::from_snapshot(&encoded(
        &json!({"observations":[observation("a",u64::MAX)]}),
    ))
    .unwrap();
    assert_eq!(
        exhausted.apply([encoded(&observation("a", u64::MAX)).as_slice()]),
        Err(Error::Sequence)
    );
}
#[test]
fn incremental_capacity_is_bounded_without_evicting_sequence_history() {
    let values: Vec<_> = (0..256)
        .map(|i| observation(&format!("id:{i}"), 1))
        .collect();
    let mut state = Observations::from_snapshot(&encoded(&json!({"observations":values}))).unwrap();
    let update = encoded(&observation("id:0", 2));
    let new = encoded(&observation("new", 1));
    assert_eq!(
        state.apply([update.as_slice(), new.as_slice()]),
        Err(Error::Limit)
    );
    assert_eq!(state.get("id:0").unwrap().sequence, 1);
    state.apply([update.as_slice()]).unwrap();
    assert_eq!(state.iter().count(), 256);
}
