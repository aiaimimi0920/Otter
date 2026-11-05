use super::*;

#[test]
fn origin_and_credentials_are_bounded_and_fail_closed() {
    for origin in [
        "http://example.com",
        "https://user:pass@example.com",
        "https://example.com/path",
        "https://example.com?token=x",
        "https://example.com#fragment",
        " https://example.com",
    ] {
        assert!(
            Endpoint::new(origin, RootCertStore::empty()).is_err(),
            "{origin}"
        );
    }
    let remote = Endpoint::new("https://example.com", RootCertStore::empty()).unwrap();
    assert!(remote
        .viewer("live:test", "device:test", (0, 0), Credential::None)
        .is_err());
    for token in ["", "with space", "bad\r\nheader", &"x".repeat(8193)] {
        assert!(remote
            .viewer(
                "live:test",
                "device:test",
                (0, 0),
                Credential::Device(token)
            )
            .is_err());
    }
    let local = Endpoint::new("http://127.0.0.1:1234", RootCertStore::empty()).unwrap();
    assert!(local
        .viewer(
            "live:test",
            "device:test",
            (0, 0),
            Credential::Device("secret")
        )
        .is_err());
    assert!(local
        .viewer("bad?query", "device:test", (0, 0), Credential::None)
        .is_err());
    assert!(local
        .viewer("live:test", "device:test", (0, 0), Credential::None)
        .is_ok());
    assert!(Endpoint::new("http://[::1]:1234", RootCertStore::empty()).is_ok());
    for origin in ["http://localhost:1234", "https://localhost:1234"] {
        assert!(Endpoint::new(origin, RootCertStore::empty())
            .unwrap()
            .viewer("live:test", "device:test", (0, 0), Credential::None)
            .is_ok());
    }
}

#[test]
fn request_matches_main_contract_and_nonce_is_fresh_and_secret_redacted() {
    let endpoint = Endpoint::new("https://example.com:9443", RootCertStore::empty()).unwrap();
    let build = || {
        endpoint
            .viewer(
                "live:test",
                "device:test",
                (7, 19),
                Credential::Device("secret-test-token"),
            )
            .unwrap()
    };
    let first = build();
    let second = build();
    assert_eq!(first.url.scheme(), "wss");
    assert_eq!(first.url.host_str(), Some("example.com"));
    assert_eq!(first.url.path(), "/v1/live/media");
    let query: std::collections::BTreeMap<_, _> = first.url.query_pairs().collect();
    assert_eq!(query.len(), 4);
    assert!(!query.contains_key("deviceId"));
    for (key, expected) in [
        ("sessionId", "live:test"),
        ("role", "viewer"),
        ("afterEpoch", "7"),
        ("afterFrameId", "19"),
    ] {
        assert_eq!(query[key], expected);
    }
    assert_eq!(
        first.request.headers()["authorization"],
        "Device secret-test-token"
    );
    assert!(!format!("{:?}", first.request).contains("secret-test-token"));
    assert_eq!(
        first.request.headers()["sec-websocket-protocol"],
        MEDIA_OFFER
    );
    let nonce = first.request.headers()["x-loom-device-nonce"]
        .to_str()
        .unwrap();
    assert_eq!(nonce.len(), 32);
    assert_ne!(
        first.request.headers()["x-loom-device-nonce"],
        second.request.headers()["x-loom-device-nonce"]
    );
    assert!(!first.url.as_str().contains("secret-test-token"));
}

#[test]
fn local_non_device_compatibility_keeps_explicit_query_identity() {
    let endpoint = Endpoint::new("https://127.0.0.1:9443", RootCertStore::empty()).unwrap();
    for credential in [Credential::None, Credential::Bearer("local-test-token")] {
        let prepared = endpoint
            .viewer("live:test", "device:local", (0, 0), credential)
            .unwrap();
        let query: std::collections::BTreeMap<_, _> = prepared.url.query_pairs().collect();
        assert_eq!(query.len(), 5);
        assert_eq!(query["deviceId"], "device:local");
        assert!(prepared
            .request
            .headers()
            .get("x-loom-device-nonce")
            .is_none());
    }
}
