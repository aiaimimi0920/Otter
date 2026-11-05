//! 清单合同夹具不读取磁盘、环境变量或实际发现凭据。
use super::*;
use serde_json::{json, Value};
fn fixture() -> Value {
    json!({"schemaVersion":1,"appId":"loom","displayName":"Loom","version":"0.2.1",
        "pid":42,"startedAt":123,"transport":{"type":"http","baseUrl":"http://127.0.0.1:8765"},
        "capabilities":["brain.plan"],"hookBridge":{"protocol":"loom.local-bridge.v1",
        "instanceId":"d2f4f738-09d9-4f46-829e-262381e63387","endpoint":"wss://127.0.0.1:43123/",
        "certificateDerBase64":"YWJj","certificateSha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "authToken":"a1".repeat(32)}})
}
fn check(value: &Value) -> Result<Identity, Error> {
    parse(&serde_json::to_vec(value).unwrap())
}
#[test]
fn accepts_main_manifest_and_remote_https_metadata_without_changing_bridge_target() {
    let mut value = fixture();
    for origin in [
        "http://127.0.0.1:8765",
        "http://[::1]:8765",
        "http://localhost:8765",
        "https://loom.example.test",
    ] {
        value["transport"]["baseUrl"] = json!(origin);
        let identity = check(&value).unwrap();
        assert_eq!(identity.address(), "127.0.0.1:43123".parse().unwrap());
        assert!(!format!("{identity:?}").contains(&"a1".repeat(32)));
    }
    value["transport"]["auth"] = json!("bearer");
    value["transport"]["authToken"] = json!("fixture-token");
    assert!(check(&value).is_ok());
}
#[test]
fn rejects_missing_or_invalid_metadata_and_capabilities() {
    for (key, bad) in [
        ("schemaVersion", json!(2)),
        ("appId", json!("other")),
        ("displayName", json!(" ")),
        ("version", json!("")),
        ("pid", Value::Null),
        ("startedAt", Value::Null),
    ] {
        let mut value = fixture();
        value[key] = bad;
        assert!(matches!(check(&value), Err(Error::Metadata)));
    }
    for capabilities in [
        json!([]),
        json!(["brain.plan", "brain.plan"]),
        json!(["brain.plan", ""]),
    ] {
        let mut value = fixture();
        value["capabilities"] = capabilities;
        assert!(matches!(check(&value), Err(Error::Capabilities)));
    }
    let mut value = fixture();
    value["hookBridge"] = Value::Null;
    assert!(matches!(check(&value), Err(Error::MissingBridge)));
}
#[test]
fn rejects_unsafe_transport_and_bridge_identity() {
    for origin in [
        "http://evil.example",
        "http://127.0.0.1.evil.example",
        "http://localhost@evil.example",
        "https://example.test/path",
        "https://example.test/?token=x",
        "https://example.test/#x",
    ] {
        let mut value = fixture();
        value["transport"]["baseUrl"] = json!(origin);
        assert!(matches!(check(&value), Err(Error::Transport)));
    }
    for auth in ["basic", "bearer"] {
        let mut value = fixture();
        value["transport"]["auth"] = json!(auth);
        assert!(matches!(check(&value), Err(Error::Transport)));
    }
    let mut value = fixture();
    value["hookBridge"]["endpoint"] = json!("ws://127.0.0.1:43123/");
    assert!(matches!(check(&value), Err(Error::Identity)));
}
#[test]
fn bounds_input_rejects_invalid_utf8_duplicate_fields_and_trailing_data() {
    assert!(matches!(
        parse(&vec![b' '; MAX_BYTES + 1]),
        Err(Error::TooLarge)
    ));
    assert!(matches!(parse(&[0xff]), Err(Error::Parse)));
    let raw = serde_json::to_string(&fixture()).unwrap();
    let duplicate = raw.replacen('{', "{\"appId\":\"loom\",", 1);
    assert!(matches!(parse(duplicate.as_bytes()), Err(Error::Parse)));
    assert!(matches!(
        parse(format!("{raw} {{}}").as_bytes()),
        Err(Error::Parse)
    ));
}
