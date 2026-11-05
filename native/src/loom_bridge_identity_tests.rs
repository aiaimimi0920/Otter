//! 摘要夹具不是真实证书：TLS 层仍必须解析证书、验证链并固定叶证书。
use super::*;
const INSTANCE: &str = "d2f4f738-09d9-4f46-829e-262381e63387";
const PIN: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
fn identity(endpoint: &str) -> Result<Identity, Error> {
    Identity::validate(
        "loom.local-bridge.v1",
        INSTANCE,
        endpoint,
        "YWJj",
        PIN,
        &"a1".repeat(32),
    )
}
#[test]
fn valid_identity_preserves_bytes_and_redacts_credentials() {
    let identity = identity("wss://127.0.0.1:43123/").unwrap();
    assert_eq!(identity.address(), "127.0.0.1:43123".parse().unwrap());
    assert_eq!(identity.certificate(), b"abc");
    assert_eq!(identity.instance(), INSTANCE);
    assert_eq!(
        identity.exporter_context(),
        format!("{INSTANCE}:{}", "a1".repeat(32))
    );
    assert_eq!(
        format!("{identity:?}"),
        "Identity { [native credentials redacted] }"
    );
    for value in [None, Some(""), Some("  "), Some(" wss://127.0.0.1:43123/ ")] {
        assert_eq!(identity.check_endpoint_override(value), Ok(()));
    }
    assert_eq!(
        identity.check_endpoint_override(Some("wss://127.0.0.1:43124/")),
        Err(Error::Endpoint)
    );
}
#[test]
fn cloned_identity_shares_verified_storage_and_survives_original_owner_drop() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Identity>();
    let original = identity("wss://127.0.0.1:43123/").unwrap();
    let worker = original.clone();
    let weak = Arc::downgrade(&original.inner);
    assert!(Arc::ptr_eq(&original.inner, &worker.inner));
    assert_eq!(
        original.certificate().as_ptr(),
        worker.certificate().as_ptr()
    );
    assert_eq!(Arc::strong_count(&original.inner), 2);
    drop(original);
    let result = std::thread::spawn(move || {
        assert_eq!(worker.instance(), INSTANCE);
        assert_eq!(worker.endpoint(), "wss://127.0.0.1:43123/");
        assert_eq!(
            worker.exporter_context(),
            format!("{INSTANCE}:{}", "a1".repeat(32))
        );
        assert!(!format!("{worker:?}").contains(&"a1".repeat(32)));
        worker.check_endpoint_override(Some("wss://127.0.0.1:43124/"))
    })
    .join()
    .unwrap();
    assert_eq!(result, Err(Error::Endpoint));
    assert!(weak.upgrade().is_none());
}
#[test]
fn endpoint_must_be_canonical_loopback_wss() {
    for endpoint in [
        "ws://127.0.0.1:80/",
        "wss://localhost:80/",
        "wss://[::1]:80/",
        "wss://192.168.1.1:80/",
        "wss://127.0.0.1:0/",
        "wss://127.0.0.1:65536/",
        "wss://127.0.0.1:080/",
        "wss://127.0.0.1:+80/",
        "wss://127.0.0.1:80/path",
        "wss://127.0.0.1:80/?key=x",
    ] {
        assert!(
            matches!(identity(endpoint), Err(Error::Endpoint)),
            "{endpoint}"
        );
    }
}
#[test]
fn malformed_identity_is_rejected_without_echoing_secrets() {
    let token = "a1".repeat(32);
    for (protocol, instance, cert, pin, credential, expected) in [
        (
            "wrong",
            INSTANCE,
            "YWJj",
            PIN,
            token.as_str(),
            Error::Protocol,
        ),
        (
            "loom.local-bridge.v1",
            "invalid",
            "YWJj",
            PIN,
            token.as_str(),
            Error::Instance,
        ),
        (
            "loom.local-bridge.v1",
            INSTANCE,
            "YWJj",
            PIN,
            "secret",
            Error::Credential,
        ),
        (
            "loom.local-bridge.v1",
            INSTANCE,
            "%%%",
            PIN,
            token.as_str(),
            Error::Certificate,
        ),
        (
            "loom.local-bridge.v1",
            INSTANCE,
            "",
            PIN,
            token.as_str(),
            Error::Certificate,
        ),
        (
            "loom.local-bridge.v1",
            INSTANCE,
            "YWJj",
            "wrong",
            token.as_str(),
            Error::Pin,
        ),
    ] {
        let error = Identity::validate(
            protocol,
            instance,
            "wss://127.0.0.1:80/",
            cert,
            pin,
            credential,
        )
        .unwrap_err();
        assert_eq!(error, expected);
        assert!(!format!("{error:?}").contains(&token));
    }
    assert!(matches!(
        Identity::validate(
            "loom.local-bridge.v1",
            INSTANCE,
            "wss://127.0.0.1:80/",
            &"A".repeat(8193),
            PIN,
            &token
        ),
        Err(Error::Certificate)
    ));
}
