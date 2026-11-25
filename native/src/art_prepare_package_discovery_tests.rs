//! Encode the current contract directly; no legacy-capability discovery or input-selected protocol downgrade.
use super::*;
#[test]
fn current_prepare_does_not_negotiate_or_preserve_requested_old_protocols() {
    for protocol in [
        "",
        "loom.art.local.prepare-package.v1",
        "future.prepare.v99",
    ] {
        let bytes =
            encode(serde_json::json!({"protocolVersion":protocol,"binding":{"requestId":"own"}}))
                .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["protocolVersion"], grant::package::visibility::PREPARE);
        assert_eq!(body["binding"]["requestId"], "own");
    }
}
#[test]
fn current_prepare_keeps_the_existing_request_byte_limit() {
    assert_eq!(
        encode(serde_json::json!({"value":"a".repeat(16 * 1024)})),
        Err(Error::Limit)
    );
}
