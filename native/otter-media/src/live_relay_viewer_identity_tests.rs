use super::*;
use serde_json::{json, Value};
#[test]
fn control_batches_are_atomic_retryable_and_bound_to_their_cursor() {
    let identity = decode(&snapshot()).unwrap().identity().clone();
    let mut cursor = ControlCursor::new(identity.clone(), 9);
    let mut response = history();
    response["events"].as_array_mut().unwrap().remove(0);
    assert!(cursor
        .prepare(&serde_json::to_vec(&response).unwrap())
        .is_err());
    assert_eq!(cursor.after(), 9);
    response["reset"] = json!(false);
    let good = serde_json::to_vec(&response).unwrap();
    response["events"][1]["sequence"] = json!(12);
    assert!(cursor
        .prepare(&serde_json::to_vec(&response).unwrap())
        .is_err());
    assert_eq!(cursor.after(), 9);
    let batch = cursor.prepare(&good).unwrap();
    assert_eq!(batch.events().len(), 2);
    assert_eq!(batch.events()[0].sequence(), 10);
    assert_eq!(batch.events()[0].kind(), "session_state");
    assert!(batch.events()[0].payload().contains("viewer_joined"));
    assert_eq!(batch.next(), 11);
    assert_eq!(cursor.after(), 9);
    let mut other = ControlCursor::new(identity, 9);
    assert_eq!(other.commit(&batch), Err(Error::History));
    cursor.commit(&batch).unwrap();
    assert_eq!(cursor.after(), 11);
    assert_eq!(cursor.commit(&batch), Err(Error::History));
    assert!(cursor.prepare(&good).is_err());
}
#[test]
fn terminal_fact_requires_member_identity_and_nonstale_epoch() {
    let identity = decode(&snapshot()).unwrap().identity().clone();
    let mut v = snapshot();
    assert_eq!(
        terminal_snapshot(&serde_json::to_vec(&v).unwrap(), &identity, "viewer:one"),
        Ok(false)
    );
    v["closed"] = json!(true);
    v["epoch"] = json!(3);
    assert_eq!(
        terminal_snapshot(&serde_json::to_vec(&v).unwrap(), &identity, "viewer:one"),
        Ok(true)
    );
    for (pointer, bad) in [
        ("/epoch", json!(1)),
        ("/session/sessionId", json!("other")),
        ("/session/viewerDevices", json!([])),
        ("/closed", json!("true")),
    ] {
        let mut bad_response = v.clone();
        *bad_response.pointer_mut(pointer).unwrap() = bad;
        assert!(terminal_snapshot(
            &serde_json::to_vec(&bad_response).unwrap(),
            &identity,
            "viewer:one"
        )
        .is_err());
    }
    assert!(terminal_snapshot(b"{\"status\":404}", &identity, "viewer:one").is_err());
}
fn snapshot() -> Value {
    json!({"session":{"protocolVersion":PROTOCOL,"sessionId":"live:test","sourceDeviceId":"source:one","sourceHookId":"hook:one","revision":7,"viewerDevices":["viewer:one"]},"epoch":2,"closed":false})
}
fn decode(v: &Value) -> Result<Snapshot, Error> {
    Snapshot::parse(&serde_json::to_vec(v).unwrap(), "live:test")
}
fn event(sequence: u64, revision: u64) -> Value {
    json!({"protocolVersion":PROTOCOL,"sessionId":"live:test","epoch":2,"sequence":sequence,"messageType":"session_state","payload":{"revision":revision,"reason":"viewer_joined","viewers":["viewer:one"]}})
}
fn history() -> Value {
    json!({"protocolVersion":PROTOCOL,"next":11,"reset":true,"events":[event(9,6),event(10,7),event(11,8)]})
}
fn cursor(v: &Value) -> Result<u64, Error> {
    join_cursor(
        &decode(&snapshot()).unwrap(),
        "viewer:one",
        &serde_json::to_vec(v).unwrap(),
    )
}
#[test]
fn snapshot_requires_exact_open_identity_and_bounded_json() {
    assert_eq!(decode(&snapshot()).unwrap().identity().epoch(), 2);
    for (pointer, bad) in [
        ("/closed", json!(true)),
        ("/closed", json!("false")),
        ("/epoch", json!(0)),
        ("/epoch", json!(-1)),
        ("/session/revision", json!(0)),
        ("/session/sessionId", json!("other")),
        ("/session/sourceHookId", json!("bad\nname")),
        ("/session/protocolVersion", json!("future")),
    ] {
        let mut v = snapshot();
        *v.pointer_mut(pointer).unwrap() = bad;
        assert!(decode(&v).is_err());
    }
    assert!(Snapshot::parse(&vec![b' '; MAX_JSON + 1], "live:test").is_err());
    let text = serde_json::to_string(&snapshot()).unwrap().replacen(
        "\"epoch\":2",
        "\"epoch\":2,\"epoch\":2",
        1,
    );
    assert!(Snapshot::parse(text.as_bytes(), "live:test").is_err());
}
#[test]
fn renewal_cannot_change_source_epoch_or_rejoin_removed_member() {
    let original = decode(&snapshot()).unwrap();
    assert!(original.renewal(original.identity(), "viewer:one").is_ok());
    for (pointer, bad) in [
        ("/session/sourceDeviceId", json!("source:two")),
        ("/session/sourceHookId", json!("hook:two")),
        ("/epoch", json!(3)),
        ("/session/viewerDevices", json!([])),
    ] {
        let mut v = snapshot();
        *v.pointer_mut(pointer).unwrap() = bad;
        assert!(decode(&v)
            .unwrap()
            .renewal(original.identity(), "viewer:one")
            .is_err());
    }
}
#[test]
fn join_cursor_replays_from_join_not_latest_and_requires_complete_suffix() {
    assert_eq!(cursor(&history()), Ok(9));
    for (pointer, bad) in [
        ("/next", json!(12)),
        ("/events/2/sequence", json!(12)),
        ("/events/2/epoch", json!(3)),
        ("/events/2/sessionId", json!("other")),
        ("/events/2/protocolVersion", json!("other")),
        ("/events/1/payload/viewers", json!([])),
        ("/events/1/payload/revision", json!(99)),
        ("/events/1/sequence", json!(0)),
    ] {
        let mut v = history();
        *v.pointer_mut(pointer).unwrap() = bad;
        assert!(cursor(&v).is_err(), "{pointer}");
    }
    let mut v = history();
    v["events"] = json!([event(11, 8)]);
    assert_eq!(cursor(&v), Err(Error::History));
}
