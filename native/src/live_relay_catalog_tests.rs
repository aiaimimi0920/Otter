use super::*;
use serde_json::{json, Value};
fn entry(id: &str, member: bool) -> Value {
    json!({"session":{"protocolVersion":"loom.live.v1","sessionId":id,"sourceDeviceId":"source:test",
        "sourceHookId":"hook:test","revision":2,"viewerDevices":if member { vec!["viewer:test"] } else { vec![] }},"epoch":1,"closed":false})
}
fn catalog(entries: Vec<Value>) -> Vec<u8> {
    serde_json::to_vec(&json!({"protocolVersion":"loom.live.v1","sessions":entries})).unwrap()
}
#[test]
fn catalog_is_sorted_bounded_and_reports_only_authenticated_selection_metadata() {
    assert!(parse(&catalog(vec![]), "viewer:test").unwrap().is_empty());
    let entries = parse(
        &catalog(vec![entry("live:z", false), entry("live:a", true)]),
        "viewer:test",
    )
    .unwrap();
    assert_eq!(entries[0].identity.session(), "live:a");
    assert_eq!(entries[0].identity.source_device(), "source:test");
    assert_eq!(entries[0].identity.source_hook(), "hook:test");
    assert_eq!(entries[0].revision, 2);
    assert!(entries[0].member);
    assert!(!entries[1].member);
}
#[test]
fn duplicate_closed_or_malformed_entries_reject_whole_catalog() {
    assert!(parse(
        &catalog(vec![entry("live:a", true), entry("live:a", false)]),
        "viewer:test"
    )
    .is_err());
    let mut closed = entry("live:b", false);
    closed["closed"] = json!(true);
    let mut missing = entry("live:b", false);
    missing["session"]
        .as_object_mut()
        .unwrap()
        .remove("viewerDevices");
    for bad in [closed, missing, entry("..", false)] {
        assert!(parse(&catalog(vec![entry("live:a", true), bad]), "viewer:test").is_err());
    }
}
#[test]
fn protocol_byte_and_entry_limits_fail_closed() {
    let entries = (0..513)
        .map(|id| entry(&format!("live:{id}"), false))
        .collect();
    assert_eq!(
        parse(&catalog(entries), "viewer:test"),
        Err(Error::Discovery)
    );
    assert!(parse(&vec![b' '; 1024 * 1024 + 1], "viewer:test").is_err());
    assert!(parse(
        br#"{"protocolVersion":"other","sessions":[]}"#,
        "viewer:test"
    )
    .is_err());
}
