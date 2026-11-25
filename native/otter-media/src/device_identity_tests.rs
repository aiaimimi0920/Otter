use super::*;

fn hex(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn legacy(seed: &[u8], public: &[u8]) -> Vec<u8> {
    serde_json::to_vec(
        &serde_json::json!({"schemaVersion":1,"deviceId":"device:test",
        "privateKey":STANDARD.encode(seed),"publicKey":STANDARD.encode(public)}),
    )
    .unwrap()
}

#[test]
fn rfc8032_vector_preserves_legacy_seed_public_key_and_signature() {
    // RFC 8032 section 7.1 TEST 3，公开测试向量，非用户凭据。
    let seed = hex("c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7");
    let public = hex("fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025");
    let expected = hex("6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a");
    let identity = Identity::parse(&legacy(&seed, &public)).unwrap();
    assert!(identity.needs_migration());
    assert_eq!(identity.device_id(), Some("device:test"));
    assert_eq!(identity.public_key().unwrap(), STANDARD.encode(public));
    assert_eq!(identity.sign(&[0xaf, 0x82]).unwrap(), expected);
    let protected = identity.protected_document().unwrap();
    assert!(!String::from_utf8_lossy(&protected).contains(&STANDARD.encode(seed)));
    let restored = Identity::parse(&protected).unwrap();
    assert!(!restored.needs_migration());
    assert_eq!(restored.device_id(), identity.device_id());
    assert_eq!(restored.sign(&[0xaf, 0x82]).unwrap(), expected);
}

#[test]
fn actual_dpapi_roundtrip_is_randomized_and_corruption_fails_closed() {
    let mut identity = Identity::generate().unwrap();
    assert_eq!(identity.device_id(), None);
    identity.bind("device:test").unwrap();
    assert!(identity.bind("device\ninjected").is_err());
    let first = identity.protected_document().unwrap();
    let second = identity.protected_document().unwrap();
    assert_ne!(first, second);
    for document in [&first, &second] {
        let restored = Identity::parse(document).unwrap();
        assert_eq!(
            restored.public_key().unwrap(),
            identity.public_key().unwrap()
        );
        assert_eq!(restored.device_id(), Some("device:test"));
    }
    let mut stored: serde_json::Value = serde_json::from_slice(&first).unwrap();
    let encrypted = stored["privateKey"]
        .as_str()
        .unwrap()
        .strip_prefix("DPAPI1:")
        .unwrap();
    let mut bytes = STANDARD.decode(encrypted).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 1;
    stored["privateKey"] = serde_json::json!(format!("DPAPI1:{}", STANDARD.encode(bytes)));
    assert!(Identity::parse(&serde_json::to_vec(&stored).unwrap()).is_err());
}

#[test]
fn malformed_identity_never_creates_a_replacement_key() {
    let identity = Identity::generate().unwrap();
    let public = STANDARD.decode(identity.public_key().unwrap()).unwrap();
    assert!(matches!(
        Identity::parse(&legacy(&[0u8; 32], &public)),
        Err(Error::Key)
    ));
    assert!(Identity::parse(&legacy(&[0u8; 31], &public)).is_err());
    assert!(matches!(
        Identity::parse(&vec![b' '; MAX_DOCUMENT + 1]),
        Err(Error::Limit)
    ));
    let mut doc: serde_json::Value =
        serde_json::from_slice(&identity.protected_document().unwrap()).unwrap();
    doc["schemaVersion"] = serde_json::json!(3);
    assert!(matches!(
        Identity::parse(&serde_json::to_vec(&doc).unwrap()),
        Err(Error::Schema)
    ));
    doc["schemaVersion"] = serde_json::json!(2);
    doc["privateKey"] = serde_json::json!(STANDARD.encode([0u8; 32]));
    assert!(matches!(
        Identity::parse(&serde_json::to_vec(&doc).unwrap()),
        Err(Error::Protection)
    ));
    assert!(Identity::parse(br#"{"schemaVersion":1,"schemaVersion":2}"#).is_err());
}
