//! 独立客户端核签名域、端口形状/总预算、离线换绑及精确保存提示；无网络或UI。
use super::*;
use package::ports::{Package, Port, Ports, Record, DOMAIN, PROTOCOL};
#[path = "art_execution_visibility_tests.rs"]
mod visibility;
fn proof() -> Package {
    Package {
        protocol_version: PROTOCOL.into(),
        package: package::Package {
            protocol_version: package::PROTOCOL.into(),
            grant: grant(),
            device_public_key: BASE64.encode([19; 32]),
            resources: vec![],
        },
        ports: Ports {
            inputs: vec![Port {
                name: "原始图片".into(),
                kind: "image".into(),
            }],
            outputs: vec![],
        },
    }
}
fn signed_proof(p: &impl serde::Serialize, domain: &[u8]) -> Vec<u8> {
    let payload = serde_json::to_vec(p).unwrap();
    serde_json::to_vec(&Envelope {
        algorithm: "ed25519".into(),
        key_id: "fixture-issuer".into(),
        signature: BASE64.encode(key().sign(&[domain, &payload].concat()).as_ref()),
        payload: BASE64.encode(payload),
    })
    .unwrap()
}
fn verify(bytes: &[u8]) -> Result<Package, Error> {
    let k = key();
    Package::verify(
        bytes,
        Issuer {
            key_id: "fixture-issuer",
            public_key: k.public_key().as_ref().try_into().unwrap(),
        },
    )
}
#[test]
fn standalone_port_proof_is_not_an_accepted_current_offline_proof() {
    let p = proof();
    let bytes = signed_proof(&p, DOMAIN);
    assert_eq!(verify(&bytes).unwrap().ports, p.ports);
    for domain in [package::DOMAIN, resume::DOMAIN, wire::DOMAIN] {
        assert!(verify(&signed_proof(&p, domain)).is_err());
    }
    let k = key();
    assert!(resume::Ticket::verify(
        &bytes,
        Issuer {
            key_id: "fixture-issuer",
            public_key: k.public_key().as_ref().try_into().unwrap(),
        },
    )
    .is_err());
    let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_slice(&BASE64.decode(&envelope.payload).unwrap()).unwrap();
    payload["ports"]["inputs"][0]["name"] = "other".into();
    envelope.payload = BASE64.encode(serde_json::to_vec(&payload).unwrap());
    assert!(verify(&serde_json::to_vec(&envelope).unwrap()).is_err());
}
#[test]
fn port_proof_rejects_invalid_names_duplicates_unknown_fields_and_shared_payload_overflow() {
    for change in 0..7 {
        let mut p = serde_json::to_value(proof()).unwrap();
        match change {
            0 => p["ports"]["inputs"][0]["name"] = " spaced ".into(),
            1 => p["ports"]["inputs"][0]["name"] = "bad\nname".into(),
            2 => p["ports"]["inputs"][0]["name"] = "a".repeat(129).into(),
            3 => {
                p["ports"]["inputs"] =
                    serde_json::json!([p["ports"]["inputs"][0], p["ports"]["inputs"][0]])
            }
            4 => p["ports"]["inputs"][0]["secret"] = true.into(),
            5 => {
                p["ports"]["inputs"] = serde_json::json!((0..33)
                    .map(|i| serde_json::json!({"name":format!("p{i}"),"type":"image"}))
                    .collect::<Vec<_>>())
            }
            _ => {
                let ports = (0..32).map(|i|serde_json::json!({"name":format!("{i:03}{}","a".repeat(125)),"type":"b".repeat(64)})).collect::<Vec<_>>();
                p["ports"] = serde_json::json!({"inputs":ports,"outputs":ports});
            }
        }
        assert!(
            verify(&signed_proof(&p, DOMAIN)).is_err(),
            "change {change}"
        );
    }
}
#[test]
fn primary_input_selection_and_persisted_record_are_bounded_and_package_specific() {
    let mut p = proof();
    p.ports.inputs.insert(
        0,
        Port {
            name: "caption".into(),
            kind: "text".into(),
        },
    );
    assert_eq!(p.ports.primary_input(), Some("原始图片"));
    let record = Record::new(&p.package.grant.binding, p.ports);
    assert!(record.validate("vendor/demo").is_ok());
    assert!(record.validate("vendor/other").is_err());
    let restored: Record = serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    assert_eq!(record, restored);
    let mut empty = Ports {
        inputs: vec![],
        outputs: vec![],
    };
    assert_eq!(empty.primary_input(), None);
    empty.inputs.push(Port {
        name: "mask".into(),
        kind: "text".into(),
    });
    assert_eq!(empty.primary_input(), Some("mask"));
}
