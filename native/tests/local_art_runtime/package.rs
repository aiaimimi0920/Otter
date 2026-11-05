//! 合成审批包及签名；仅测试 wire 生产者，预期像素由结果端独立常量核对。
use super::native::art_execution_grant::wire;
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
pub const ART: &str = "vendor/product-package";
pub const SERVICE: &str = "capability:vendor:service.run";
pub const SOURCE: &str = include_str!("source.js");
pub fn key() -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[47; 32]).unwrap()
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn permissions() -> Value {
    json!(["local_state", "preview", "loom_service", "workspace_result"])
}
pub fn settings(origin: &str) -> Value {
    json!({"schemaVersion":2,"artIssuerTrust":{"issuers":[{
        "origin":origin,"keyId":"product-issuer","publicKey":STANDARD.encode(key().public_key().as_ref())}]}})
}
pub fn resource(id: &str) -> Vec<u8> {
    match id {
        "config" => br#"{"answer":42}"#.to_vec(),
        "binary" => vec![0, 1, 2, 255],
        "picture" => {
            let mut bytes = Vec::new();
            let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .unwrap()
                .write_image_data(&[
                    255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 128,
                ])
                .unwrap();
            bytes
        }
        _ => panic!("unknown product fixture resource"),
    }
}
fn signed(domain: &[u8], value: &Value) -> Value {
    let payload = serde_json::to_vec(value).unwrap();
    let signature = key().sign(&[domain, payload.as_slice()].concat());
    json!({"algorithm":"ed25519","keyId":"product-issuer","payload":STANDARD.encode(payload),
        "signature":STANDARD.encode(signature.as_ref())})
}
pub fn prepare_source(body: &Value, origin: &str, device_key: &str, source: &str) -> Value {
    assert_eq!(body["protocolVersion"], "loom.art.local.prepare-package.v1");
    assert_eq!(body["permissions"], permissions());
    assert_eq!(body["services"], json!([SERVICE]));
    let binding: wire::Binding = serde_json::from_value(body["binding"].clone()).unwrap();
    assert_eq!(binding.origin, origin);
    assert_eq!(binding.device_id, "device-product-art");
    assert_eq!(binding.art_id, ART);
    assert_eq!(binding.package_digest, "b".repeat(64));
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let hash = digest(source.as_bytes());
    let grant = json!({"protocolVersion":wire::PROTOCOL,"grantId":"grant:product-art","binding":binding,
        "executionOwner":"otter","resultOwner":"otter_workspace","permissions":permissions(),"services":[SERVICE],
        "issuedAtMs":time-1,"expiresAtMs":time+120000,"offlineUntilMs":time+120000,
        "source":{"resourceId":format!("sha256:{hash}"),"digest":hash,"size":source.len()}});
    let resources: Vec<_> = [
        ("config", "application/json"),
        ("binary", "application/octet-stream"),
        ("picture", "image/png"),
    ]
    .into_iter()
    .map(|(id, mime)| {
        let bytes = resource(id);
        json!({"id":id,"mimeType":mime,"size":bytes.len(),"digest":digest(&bytes),
                "dimensions":if id=="picture" {[2,2]} else {[0,0]}})
    })
    .collect();
    json!({"protocolVersion":"loom.art.local.prepare-package.v1","source":STANDARD.encode(source),
        "grant":signed(wire::DOMAIN,&grant),"package":signed(b"loom.art.local.package.v1\0",&json!({
            "protocolVersion":"loom.art.local.package.v1","grant":grant,"devicePublicKey":device_key,"resources":resources}))})
}
