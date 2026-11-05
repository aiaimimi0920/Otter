use super::*;
pub(crate) fn fixture() -> Value {
    serde_json::json!({"protocolVersion":"loom.hook.v1","serverName":"Loom","serverVersion":"1.0.0",
        "sessionId":"fixture-session","transport":"websocket","capabilities":{
        "artDefinitions":[{"id":"test-art","label":"测试","parameters":[{"id":"secret","secret":true}],
            "inputs":[],"outputs":[],"defaults":{"amount":2},"metadata":{"category":"test"}}],
        "operations":["loom.hook.art.execute"],"surface":{"apiVersion":"1",
        "runtimes":[],"nodes":[],"transports":[],"capabilities":[],
        "input":{"pointer":false,"hover":false,"touch":false,"keyboard":false}}}})
}
fn parse(value: &Value) -> Result<Option<Handshake>, Error> {
    response(&serde_json::to_vec(value).unwrap())
}
#[test]
fn request_and_catalog_preserve_protocol_and_definition_fields() {
    let wire: Value = serde_json::from_str(&request("otter-native", "0.2.33").unwrap()).unwrap();
    assert_eq!(wire["method"], "loom.hook.handshake");
    assert_eq!(
        wire["params"]["transports"],
        serde_json::json!(["websocket", "shared_memory"])
    );
    let result = parse(&fixture()).unwrap().unwrap();
    assert_eq!(result.arts()[0]["parameters"][0]["secret"], true);
    assert_eq!(result.arts()[0]["defaults"]["amount"], 2);
    assert_eq!(result.session_id(), "fixture-session");
    assert_eq!(result.transport(), Transport::Websocket);
    let capability = result.capability(0).unwrap();
    assert!(capability.enabled);
    let secret = capability.parameters().next().unwrap();
    assert!(secret.secret);
    assert!(secret.default.is_none());
}
#[test]
fn catalog_identity_uses_normalized_ids_and_missing_label_falls_back() {
    let mut value = fixture();
    value["capabilities"]["artDefinitions"][0]
        .as_object_mut()
        .unwrap()
        .remove("label");
    let result = parse(&value).unwrap().unwrap();
    assert_eq!(result.capability(0).unwrap().label, "test-art");
    let mut duplicate = value["capabilities"]["artDefinitions"][0].clone();
    duplicate["id"] = " test-art ".into();
    value["capabilities"]["artDefinitions"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(matches!(parse(&value), Err(Error::Capabilities)));
}
#[test]
fn invalid_or_oversized_catalog_is_not_accepted() {
    assert!(matches!(
        response(&vec![b' '; MAX_BYTES + 1]),
        Err(Error::TooLarge)
    ));
    assert!(response(b"{bad").is_err());
    assert!(
        parse(&serde_json::json!({"method":"loom.hook.capabilities.updated"}))
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        parse(&serde_json::json!({"error":"private detail"})),
        Err(Error::Remote)
    ));
    for field in ["protocolVersion", "sessionId", "transport"] {
        let mut value = fixture();
        value[field] = Value::String(String::new());
        assert!(parse(&value).is_err());
    }
    let mut value = fixture();
    let art = value["capabilities"]["artDefinitions"][0].clone();
    value["capabilities"]["artDefinitions"]
        .as_array_mut()
        .unwrap()
        .push(art);
    assert!(matches!(parse(&value), Err(Error::Capabilities)));
    let mut value = fixture();
    value["capabilities"]["artDefinitions"][0]["parameters"] =
        serde_json::json!(vec![serde_json::json!({}); 129]);
    assert!(matches!(parse(&value), Err(Error::Capabilities)));
    let wire = serde_json::to_string(&fixture()).unwrap();
    let duplicate = wire.replacen("{", "{\"sessionId\":\"duplicate\",", 1);
    assert!(matches!(
        response(duplicate.as_bytes()),
        Err(Error::Encoding)
    ));
}
