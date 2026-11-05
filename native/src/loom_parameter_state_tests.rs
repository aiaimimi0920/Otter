use super::*;
use serde_json::json;
fn definition() -> Value {
    json!({"id":"art","parameters":[
        {"id":"number","type":"number","default":"12"},
        {"id":"missing"},{"id":"null","default":null},
        {"id":"secret","type":"secret","default":"private"},
        {"id":"__exec_internal","default":false}
    ]})
}
#[test]
fn public_initial_state_owns_values_and_distinguishes_undefined_from_null() {
    let params = {
        let raw = definition();
        Parameters::new(&Capability::new(&raw).unwrap()).unwrap()
    };
    assert_eq!(params.art_id(), "art");
    assert_eq!(params.revision(), 0);
    assert_eq!(params.get("number"), Some(Some(&json!(12.0))));
    assert_eq!(params.get("missing"), Some(None));
    assert_eq!(params.get("null"), Some(Some(&Value::Null)));
    assert!(params.get("secret").is_none());
    assert!(params.get("__exec_internal").is_none());
}
#[test]
fn edits_are_versioned_only_when_changed_and_unknown_or_secret_names_are_rejected() {
    let raw = definition();
    let mut params = Parameters::new(&Capability::new(&raw).unwrap()).unwrap();
    assert!(!params.set("number", Some(json!(12.0))).unwrap());
    assert!(params.set("number", Some(json!(13))).unwrap());
    assert_eq!(params.revision(), 1);
    assert!(params.set("missing", Some(Value::Null)).unwrap());
    assert_eq!(params.get("missing"), Some(Some(&Value::Null)));
    for id in ["secret", "__exec_internal", "unknown"] {
        assert_eq!(
            params.set(id, Some(json!(true))),
            Err(Error::UnknownParameter)
        );
    }
    assert_eq!(params.revision(), 2);
    assert!(params.set("missing", None).unwrap());
    assert_eq!(params.get("missing"), Some(None));
}
#[test]
fn rejected_edits_preserve_values_and_revision() {
    let raw = definition();
    let mut params = Parameters::new(&Capability::new(&raw).unwrap()).unwrap();
    for value in [json!("x".repeat(MAX_BYTES)), json!(vec![0; 4096])] {
        assert_eq!(params.set("number", Some(value)), Err(Error::Capacity));
        assert_eq!(params.get("number"), Some(Some(&json!(12.0))));
        assert_eq!(params.revision(), 0);
    }
    params.revision = u64::MAX;
    assert_eq!(
        params.set("number", Some(json!(1))),
        Err(Error::RevisionExhausted)
    );
    assert_eq!(params.get("number"), Some(Some(&json!(12.0))));
}
#[test]
fn initial_values_are_checked_before_copying() {
    let mut raw = definition();
    raw["parameters"][0]["default"] = json!("x".repeat(MAX_BYTES));
    assert!(matches!(
        Parameters::new(&Capability::new(&raw).unwrap()),
        Err(Error::Capacity)
    ));
    raw["parameters"][0]["default"] = Value::Null;
    raw["parameters"][0]["id"] = "bad\nname".into();
    assert!(matches!(
        Parameters::new(&Capability::new(&raw).unwrap()),
        Err(Error::Capacity)
    ));
}
