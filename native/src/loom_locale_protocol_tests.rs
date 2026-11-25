use super::*;
use serde_json::{json, Value};

fn envelope() -> Value {
    json!({
        "loomId": "loom-a",
        "deviceId": "device-a",
        "locale": {
            "receiverGeneration": "generation-a",
            "language": "en",
            "revision": 7,
            "supportedLanguages": ["en", "zh-Hans"],
            "active": true,
            "request": {
                "requestId": "request-a",
                "expectedRevision": 7,
                "language": "zh-Hans",
                "receiverGeneration": "generation-a",
                "expiresAt": 9999,
                "validForMs": 12000
            }
        }
    })
}

fn parse(value: &Value) -> Result<Snapshot, &'static str> {
    Snapshot::parse(
        &serde_json::to_vec(value).unwrap(),
        "loom-a",
        "device-a",
        Some("generation-a"),
    )
}

#[test]
fn locale_wire_binds_actual_device_loom_generation_and_language() {
    assert!(parse(&envelope()).is_ok());
    for (path, value) in [
        ("/loomId", json!("loom-b")),
        ("/deviceId", json!("device-b")),
        ("/locale/receiverGeneration", json!("generation-b")),
        ("/locale/active", json!(false)),
        ("/locale/language", json!("unknown")),
        ("/locale/revision", json!(MAX_REVISION + 1)),
        ("/locale/supportedLanguages", json!(["en", "en"])),
        ("/locale/request/receiverGeneration", json!("generation-b")),
        ("/locale/request/expectedRevision", json!(MAX_REVISION + 1)),
        ("/locale/request/language", json!("../../file")),
        ("/locale/request/requestId", json!("bad\nid")),
        ("/locale/request/validForMs", json!(0)),
        ("/locale/request/validForMs", json!(15001)),
    ] {
        let mut wire = envelope();
        *wire.pointer_mut(path).unwrap() = value;
        assert!(parse(&wire).is_err(), "{path}");
    }
}

#[test]
fn locale_request_deadline_uses_monotonic_request_start_not_remote_clock() {
    let snapshot = parse(&envelope()).unwrap();
    let request = snapshot.request.unwrap();
    let started = Instant::now();
    assert_eq!(
        request.deadline(started),
        Some(started + Duration::from_secs(12))
    );
}

#[test]
fn locale_response_is_bounded_and_never_uses_an_unknown_language() {
    assert!(Snapshot::parse(&vec![b' '; MAX_JSON + 1], "", "", None).is_err());
    let mut value = envelope();
    value["locale"]["request"] = Value::Null;
    assert!(parse(&value).is_ok());
    let mut request = envelope()["locale"]["request"].clone();
    request["path"] = json!("app-settings.json");
    value["locale"]["request"] = request;
    assert!(
        parse(&value).is_err(),
        "settings paths must not cross this boundary"
    );
}
