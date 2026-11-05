use super::*;
use ring::signature::{UnparsedPublicKey, ED25519};

fn identity() -> Identity {
    let mut identity = Identity::generate().unwrap();
    identity.bind("device:test").unwrap();
    identity
}
fn challenge(device: &str, id: &str, value: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"deviceId":device,"challengeId":id,"challenge":value}))
        .unwrap()
}

#[test]
fn challenge_proof_signs_exact_main_domain_separator_with_fresh_nonce() {
    let identity = identity();
    let body = challenge("device:test", "challenge:test", "challenge-value");
    let first: serde_json::Value =
        serde_json::from_slice(&create(&identity, &body).unwrap()).unwrap();
    let second: serde_json::Value =
        serde_json::from_slice(&create(&identity, &body).unwrap()).unwrap();
    assert_ne!(first["clientNonce"], second["clientNonce"]);
    for proof in [first, second] {
        assert_eq!(proof["deviceId"], "device:test");
        assert_eq!(proof["challengeId"], "challenge:test");
        let nonce = proof["clientNonce"].as_str().unwrap();
        assert_eq!(URL_SAFE_NO_PAD.decode(nonce).unwrap().len(), 24);
        let message = format!(
            "loom.device-session.v1\ndevice:test\nchallenge:test\nchallenge-value\n{nonce}"
        );
        let public = STANDARD.decode(identity.public_key().unwrap()).unwrap();
        let signature = STANDARD
            .decode(proof["signature"].as_str().unwrap())
            .unwrap();
        let verifier = UnparsedPublicKey::new(&ED25519, &public);
        verifier.verify(message.as_bytes(), &signature).unwrap();
        assert!(verifier.verify(b"other-domain", &signature).is_err());
        assert!(proof.get("privateKey").is_none());
    }
}

#[test]
fn unpaired_foreign_malformed_and_oversized_challenges_are_rejected() {
    let identity = identity();
    assert!(matches!(
        create(&identity, &challenge("other", "id", "value")),
        Err(Error::Identity)
    ));
    assert!(matches!(
        create(
            &Identity::generate().unwrap(),
            &challenge("device:test", "id", "value")
        ),
        Err(Error::Identity)
    ));
    for (id, value) in [
        ("", "value"),
        ("id\ninjected", "value"),
        ("id", "value\ninjected"),
        ("id", ""),
    ] {
        assert!(matches!(
            create(&identity, &challenge("device:test", id, value)),
            Err(Error::Challenge)
        ));
    }
    assert!(matches!(
        create(&identity, &vec![b' '; 256 * 1024 + 1]),
        Err(Error::Limit)
    ));
    assert!(create(
        &identity,
        br#"{"deviceId":"device:test","deviceId":"other"}"#
    )
    .is_err());
}
