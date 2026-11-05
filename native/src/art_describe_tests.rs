//! 描述不是许可；严格字段/范围验证、宿主身份注入与原租约撤销保护。
use super::*;
use crate::art_local_requests::tests as fixture;
use serde_json::{json, Value};
const ORIGIN: &str = "https://loom.example";
#[test]
#[ignore = "explicit real Loom HTTP descriptor fixture input"]
fn consumes_real_loom_http_descriptor_without_trusting_response_identity() {
    use std::io::Read;
    let path =
        std::env::var_os("OTTER_LOCAL_ART_DESCRIPTOR_FIXTURE").expect("explicit Loom fixture");
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .unwrap()
        .take(16 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    let mut owner = fixture::owner();
    let descriptor = Descriptor::accept(
        &bytes,
        "publisher.test/local-counter",
        ORIGIN,
        fixture::lease(&mut owner),
    )
    .unwrap();
    let (request, _) = descriptor
        .into_request("workspace:consumer", "instance:consumer", 3)
        .unwrap();
    assert_eq!(request.binding.art_id, "publisher.test/local-counter");
    assert_eq!(request.binding.art_version, "1.0.0");
    assert_eq!(request.binding.origin, ORIGIN);
    assert_eq!(request.binding.device_id, "device:1");
    assert_eq!(request.binding.workspace_id, "workspace:consumer");
    assert_eq!(
        request.permissions,
        vec![wire::Permission::LocalState, wire::Permission::Preview]
    );
}
fn response() -> Value {
    json!({"protocolVersion":"loom.art.local.describe.v1","descriptor":{
        "artId":"vendor/demo","artVersion":"1.0.0","packageDigest":"a".repeat(64),
        "sdkVersion":"otter.art.v1","permissions":["local_state","preview"],"services":[]}})
}
#[test]
fn descriptor_uses_host_identity_and_preserves_original_lease() {
    let mut owner = fixture::owner();
    let lease = fixture::lease(&mut owner);
    let descriptor = Descriptor::accept(
        &serde_json::to_vec(&response()).unwrap(),
        "vendor/demo",
        ORIGIN,
        lease.clone(),
    )
    .unwrap();
    let (request, returned) = descriptor
        .into_request("workspace:host", "instance:host", 17)
        .unwrap();
    assert!(Arc::ptr_eq(&lease, &returned));
    assert_eq!(request.binding.origin, ORIGIN);
    assert_eq!(request.binding.device_id, "device:1");
    assert_eq!(request.binding.workspace_id, "workspace:host");
    assert_eq!(request.binding.instance_id, "instance:host");
    assert_eq!(request.binding.generation, 17);
    assert_eq!(request.binding.request_id, "pending");
}
#[test]
fn descriptor_rejects_unrequested_identity_and_unsafe_contract() {
    let mut owner = fixture::owner();
    assert!(Descriptor::accept(
        &vec![b' '; 16 * 1024 + 1],
        "vendor/demo",
        ORIGIN,
        fixture::lease(&mut owner)
    )
    .is_err());
    for (field, value) in [
        ("artId", json!("vendor/other")),
        ("artVersion", json!("../escape")),
        ("packageDigest", json!("not-a-digest")),
        ("sdkVersion", json!("future")),
        ("permissions", json!([])),
        ("permissions", json!(["preview", "preview"])),
        ("permissions", json!(["preview", "loom_service"])),
        ("services", json!(["service:extra"])),
        ("origin", json!("https://evil.example")),
        ("publicKey", json!("peer-selected")),
    ] {
        let mut data = response();
        data["descriptor"][field] = value;
        assert!(
            Descriptor::accept(
                &serde_json::to_vec(&data).unwrap(),
                "vendor/demo",
                ORIGIN,
                fixture::lease(&mut owner)
            )
            .is_err(),
            "{field}"
        );
    }
    let mut data = response();
    data["protocolVersion"] = json!("unknown");
    assert!(Descriptor::accept(
        &serde_json::to_vec(&data).unwrap(),
        "vendor/demo",
        ORIGIN,
        fixture::lease(&mut owner)
    )
    .is_err());
    let bytes = serde_json::to_string(&response())
        .unwrap()
        .replace("\"artId\":", "\"artId\":\"vendor/demo\",\"artId\":");
    assert!(Descriptor::accept(
        bytes.as_bytes(),
        "vendor/demo",
        ORIGIN,
        fixture::lease(&mut owner)
    )
    .is_err());
}
#[test]
fn revoked_description_or_invalid_host_context_cannot_become_a_request() {
    let mut owner = fixture::owner();
    let bytes = serde_json::to_vec(&response()).unwrap();
    for (workspace, generation) in [("", 1), ("workspace:host", 0), ("workspace:host", u64::MAX)] {
        let descriptor =
            Descriptor::accept(&bytes, "vendor/demo", ORIGIN, fixture::lease(&mut owner)).unwrap();
        assert!(descriptor
            .into_request(workspace, "instance:host", generation)
            .is_err());
    }
    let lease = fixture::lease(&mut owner);
    let descriptor = Descriptor::accept(&bytes, "vendor/demo", ORIGIN, lease.clone()).unwrap();
    lease.revoke();
    assert!(matches!(
        descriptor.into_request("workspace:host", "instance:host", 1),
        Err(Error::Lease)
    ));
}
