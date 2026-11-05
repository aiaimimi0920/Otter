//! Test-only signed Art publisher + issuer, independent of viewer output and pixel oracle.
use super::*;
use crate::art_execution_grant::{package, wire};
use base64::{engine::general_purpose::STANDARD, Engine};
use ring::signature::{Ed25519KeyPair, KeyPair};
use sha2::{Digest, Sha256};
const SOURCE: &str = "NeuroSurface.define({mount({root}){root.textContent='native viewer host'}});";
fn key() -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[59; 32]).unwrap()
}
fn signed<T: serde::Serialize>(value: &T, domain: &[u8]) -> Value {
    let payload = serde_json::to_vec(value).unwrap();
    let signature = key().sign(&[domain, payload.as_slice()].concat());
    json!({"algorithm":"ed25519","keyId":"viewer-art-issuer","payload":STANDARD.encode(payload),"signature":STANDARD.encode(signature.as_ref())})
}
pub(super) fn authority(origin: &str) -> crate::art_issuer_trust::Authority {
    crate::art_issuer_trust::Authority::new(crate::art_issuer_trust::Document {
        issuers: vec![crate::art_issuer_trust::Entry {
            origin: origin.into(),
            key_id: "viewer-art-issuer".into(),
            public_key: STANDARD.encode(key().public_key().as_ref()),
        }],
    })
    .unwrap()
}
pub(super) fn mount(listener: &TcpListener, tls: &Arc<ServerConfig>) {
    for stage in 0..2 {
        let mut stream = StreamOwned::new(
            ServerConnection::new(tls.clone()).unwrap(),
            accept(listener),
        );
        let (head, body) = request(&mut stream);
        assert!(head
            .to_ascii_lowercase()
            .contains("authorization: device configured-token"));
        let result = if stage == 0 {
            assert!(head.starts_with("POST /v1/art/local/describe "));
            assert_eq!(body, json!({"artId":"vendor/viewer"}));
            json!({"protocolVersion":"loom.art.local.describe.v1","descriptor":{"artId":"vendor/viewer","artVersion":"1.0.0","packageDigest":"b".repeat(64),"sdkVersion":wire::SDK,"permissions":["local_state","preview"],"services":[]}})
        } else {
            assert!(head.starts_with("POST /v3/art/local/prepare-package "));
            let binding: wire::Binding = serde_json::from_value(body["binding"].clone()).unwrap();
            assert_eq!(binding.device_id, "device-old");
            assert_eq!(binding.art_id, "vendor/viewer");
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let digest = format!("{:x}", Sha256::digest(SOURCE));
            let grant = wire::Grant {
                protocol_version: wire::PROTOCOL.into(),
                grant_id: "grant:native-viewer".into(),
                binding,
                execution_owner: "otter".into(),
                result_owner: "otter_workspace".into(),
                permissions: vec![wire::Permission::LocalState, wire::Permission::Preview],
                services: vec![],
                issued_at_ms: time - 1,
                expires_at_ms: time + 60000,
                offline_until_ms: 0,
                source: wire::Source {
                    resource_id: format!("sha256:{digest}"),
                    digest,
                    size: SOURCE.len() as u64,
                },
            };
            let proof = package::Package {
                protocol_version: package::PROTOCOL.into(),
                device_public_key: String::new(),
                grant: grant.clone(),
                resources: vec![],
            };
            let proof = package::visibility::Package {
                protocol_version: package::visibility::PROTOCOL.into(),
                package: proof,
                ports: package::ports::Ports {
                    inputs: vec![],
                    outputs: vec![],
                },
                visibility: package::visibility::Visibility {
                    inputs: vec![],
                    outputs: vec![],
                },
            };
            json!({"protocolVersion":package::visibility::PREPARE,"grant":signed(&grant,wire::DOMAIN),"source":STANDARD.encode(SOURCE),"package":signed(&proof,package::visibility::DOMAIN)})
        };
        reply(&mut stream, 200, &serde_json::to_vec(&result).unwrap());
    }
}
pub(super) fn check_join(body: &Value) {
    assert_eq!(body["protocolVersion"], "loom.art.local.live-viewer.v1");
    assert_eq!(body["requireExistingMembership"], false);
    check_proof(body);
}
pub(super) fn check_control(body: &Value) {
    assert_eq!(body["protocolVersion"], "loom.art.local.live-control.v1");
    assert!(body.get("requireExistingMembership").is_none());
    assert!(matches!(
        body["operation"]["kind"].as_str(),
        Some("acquire" | "release" | "input")
    ));
    check_proof(body);
}
fn check_proof(body: &Value) {
    assert!(body.get("surfaceInstanceId").is_none());
    assert!(body.get("attachmentId").is_none());
    let envelope: wire::Envelope = serde_json::from_value(body["grant"].clone()).unwrap();
    let payload = STANDARD.decode(&envelope.payload).unwrap();
    ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, key().public_key().as_ref())
        .verify(
            &[wire::DOMAIN, payload.as_slice()].concat(),
            &STANDARD.decode(envelope.signature).unwrap(),
        )
        .unwrap();
    let grant: wire::Grant = serde_json::from_slice(&payload).unwrap();
    assert_eq!(
        body["binding"],
        serde_json::to_value(grant.binding).unwrap()
    );
}
