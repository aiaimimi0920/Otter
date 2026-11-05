use super::*;
fn raw(value: serde_json::Value) -> Box<RawValue> {
    RawValue::from_string(value.to_string()).unwrap()
}
fn resource() -> serde_json::Value {
    serde_json::json!({"kind":"resource","resource":{"resourceId":format!("sha256:{}", "a".repeat(64)),
        "kind":"image","mime":"image/png","size":123,"width":640,"height":480}})
}
#[test]
fn typed_ports_borrow_json_and_accept_valid_resource_and_stream_descriptors() {
    let raw = raw(
        serde_json::json!({"value":{"kind":"value","value":{"hello":"world"}},
        "image":resource(),"stream":{"kind":"stream","stream":{"streamId":"stream:test","itemType":"frame","sequence":0}}}),
    );
    let ports = parse(&raw).unwrap();
    assert_eq!(ports.len(), 3);
    let value = ports.iter().find(|p| p.id == "value").unwrap();
    let Value::Value { value } = &value.value else {
        panic!("wrong type")
    };
    let start = raw.get().as_ptr() as usize;
    assert!((start..start + raw.get().len()).contains(&(value.get().as_ptr() as usize)));
    let Value::Resource { resource } = &ports.iter().find(|p| p.id == "image").unwrap().value
    else {
        panic!("wrong type")
    };
    assert_eq!(resource.kind, ResourceKind::Image);
    assert_eq!(resource.width, Some(640));
}
#[test]
fn malformed_resources_reject_bad_hash_dimensions_and_unsafe_integer() {
    for (field, value) in [
        ("resourceId", serde_json::json!("file:///private")),
        (
            "resourceId",
            serde_json::json!(format!("sha256:{}", "A".repeat(64))),
        ),
        ("kind", serde_json::json!("unknown")),
        ("mime", serde_json::json!("x".repeat(129))),
        ("size", serde_json::json!(MAX_SAFE_INTEGER + 1)),
        ("size", serde_json::json!(-1)),
        ("width", serde_json::json!(0)),
        ("height", serde_json::json!(16385)),
    ] {
        let mut port = resource();
        port["resource"][field] = value;
        assert!(
            parse(&raw(serde_json::json!({"image":port}))).is_err(),
            "{field}"
        );
    }
    assert!(parse(&raw(
        serde_json::json!({"image":{"kind":"resource","resource":{}}})
    ))
    .is_err());
}
#[test]
fn stream_descriptors_validate_identity_limits_and_sequence() {
    let stream = serde_json::json!({"streamId":"stream:test","itemType":"frame","mime":"image/png","sequence":1});
    for (field, value) in [
        ("streamId", serde_json::json!("bad\n")),
        ("itemType", serde_json::json!("x".repeat(161))),
        ("mime", serde_json::json!("x".repeat(129))),
        ("sequence", serde_json::json!(MAX_SAFE_INTEGER + 1)),
        ("sequence", serde_json::json!(1.5)),
    ] {
        let mut value_stream = stream.clone();
        value_stream[field] = value;
        assert!(parse(&raw(
            serde_json::json!({"stream":{"kind":"stream","stream":value_stream}})
        ))
        .is_err());
    }
}
#[test]
fn null_value_is_not_missing_and_escaped_duplicate_port_names_are_rejected() {
    let valid =
        RawValue::from_string(r#"{"\u0078":{"kind":"value","value":null}}"#.into()).unwrap();
    let ports = parse(&valid).unwrap();
    assert_eq!(ports[0].id, "x");
    let Value::Value { value } = &ports[0].value else {
        panic!("wrong type")
    };
    assert_eq!(value.get(), "null");
    for invalid in [
        r#"{"x":{"kind":"value"}}"#,
        r#"{"x":{"kind":"value","value":null},"\u0078":{"kind":"value","value":null}}"#,
    ] {
        assert!(parse(&RawValue::from_string(invalid.into()).unwrap()).is_err());
    }
}
