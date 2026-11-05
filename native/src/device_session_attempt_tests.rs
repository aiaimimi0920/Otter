use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{UnparsedPublicKey, ED25519};

fn identity() -> Identity {
    let mut identity = Identity::generate().unwrap();
    identity.bind("device-1").unwrap();
    identity
}
fn signed(identity: &Identity) -> SignedAttempt {
    Attempt::new(identity, "https://loom.example")
        .unwrap()
        .challenge(
            200,
            br#"{"deviceId":"device-1","challengeId":"id","challenge":"challenge"}"#,
        )
        .unwrap()
}
fn response(device: &str, token: &str, expiry: u64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"deviceId":device,"token":token,"expiresAtMs":expiry}))
        .unwrap()
}

#[test]
fn signed_exchange_is_main_compatible_and_token_is_origin_and_expiry_bound() {
    let identity = identity();
    let attempt = Attempt::new(&identity, "https://LOOM.example:443/").unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&attempt.challenge_body()).unwrap(),
        serde_json::json!({"deviceId":"device-1"})
    );
    let signed = attempt
        .challenge(
            201,
            br#"{"deviceId":"device-1","challengeId":"id","challenge":"challenge"}"#,
        )
        .unwrap();
    let proof: serde_json::Value = serde_json::from_slice(signed.body()).unwrap();
    let nonce = proof["clientNonce"].as_str().unwrap();
    let message = format!("loom.device-session.v1\ndevice-1\nid\nchallenge\n{nonce}");
    UnparsedPublicKey::new(
        &ED25519,
        STANDARD.decode(identity.public_key().unwrap()).unwrap(),
    )
    .verify(
        message.as_bytes(),
        &STANDARD
            .decode(proof["signature"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let session = signed
        .finish(201, &response("device-1", "secret", 101), 100)
        .unwrap();
    assert_eq!(session.device_id(), "device-1");
    assert_eq!(session.expires_at_ms(), 101);
    assert_eq!(session.token_for("https://loom.example", 100), Ok("secret"));
    for origin in [
        "https://other.example",
        "https://loom.example:444",
        "http://loom.example",
    ] {
        assert_eq!(session.token_for(origin, 100), Err(Error::Origin));
    }
    assert_eq!(
        session.token_for("https://loom.example", 101),
        Err(Error::Expired)
    );
}

#[test]
fn only_challenge_403_exact_code_is_pending_and_no_error_echoes_body() {
    let identity = identity();
    let body = br#"{"error":{"code":"device_not_authorized","message":"sensitive"}}"#;
    for (status, expected) in [
        (403, Error::PendingApproval),
        (401, Error::Http(401)),
        (429, Error::Http(429)),
        (302, Error::Http(302)),
        (500, Error::Http(500)),
    ] {
        let err = Attempt::new(&identity, "https://loom.example")
            .unwrap()
            .challenge(status, body)
            .err()
            .unwrap();
        assert_eq!(err, expected);
        assert!(!format!("{err:?}").contains("sensitive"));
    }
    for body in [
        br#"{"error":{"code":"revoked"}}"#.as_slice(),
        b"bad json",
        br#"{"error":{"code":"revoked","code":"device_not_authorized"}}"#,
    ] {
        assert_eq!(
            Attempt::new(&identity, "https://loom.example")
                .unwrap()
                .challenge(403, body)
                .err(),
            Some(Error::Http(403))
        );
    }
    assert_eq!(
        signed(&identity).finish(403, body, 0).err(),
        Some(Error::Http(403))
    );
}

#[test]
fn session_rejects_wrong_identity_unsafe_tokens_expiry_and_duplicate_fields() {
    let identity = identity();
    assert_eq!(
        signed(&identity)
            .finish(200, &response("other", "secret", 10), 0)
            .err(),
        Some(Error::Identity)
    );
    for token in ["", " ", "secret\r\nInjected: yes", "token space", "非ASCII"] {
        assert_eq!(
            signed(&identity)
                .finish(200, &response("device-1", token, 10), 0)
                .err(),
            Some(Error::Credential)
        );
    }
    assert_eq!(
        signed(&identity)
            .finish(200, &response("device-1", &"x".repeat(8193), 10), 0)
            .err(),
        Some(Error::Credential)
    );
    for expiry in [0, 99, 100] {
        assert_eq!(
            signed(&identity)
                .finish(200, &response("device-1", "secret", expiry), 100)
                .err(),
            Some(Error::Expired)
        );
    }
    for body in [
        br#"{"deviceId":"device-1","token":"secret","token":"second","expiresAtMs":10}"#.as_slice(),
        br#"{"deviceId":"device-1","token":"secret","expiresAtMs":-1}"#,
        b"null",
    ] {
        assert_eq!(
            signed(&identity).finish(200, body, 0).err(),
            Some(Error::Json)
        );
    }
}

#[test]
fn unpaired_invalid_origins_and_oversized_responses_fail_closed() {
    assert!(matches!(
        Attempt::new(&Identity::generate().unwrap(), "https://loom.example"),
        Err(Error::Identity)
    ));
    let identity = identity();
    for origin in [
        "http://localhost",
        "https://user@loom.example",
        "https://loom.example/a",
        "https://loom.example?query",
        "https://loom.example#fragment",
        " https://loom.example",
    ] {
        assert!(matches!(
            Attempt::new(&identity, origin),
            Err(Error::Origin)
        ));
    }
    let body = vec![b' '; MAX_RESPONSE + 1];
    assert_eq!(
        Attempt::new(&identity, "https://loom.example")
            .unwrap()
            .challenge(403, &body)
            .err(),
        Some(Error::Limit)
    );
    assert_eq!(
        signed(&identity).finish(200, &body, 0).err(),
        Some(Error::Limit)
    );
    assert!(matches!(
        Attempt::new(&identity, "https://loom.example")
            .unwrap()
            .challenge(
                200,
                br#"{"deviceId":"other","challengeId":"id","challenge":"challenge"}"#
            ),
        Err(Error::Proof(device_session_proof::Error::Identity))
    ));
}
