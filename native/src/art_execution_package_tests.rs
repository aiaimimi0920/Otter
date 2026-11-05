//! 独立客户端资源证明验证；签名预期由固定测试issuer直接产生，不调用待测包验签器。
use super::package::{Package, Resource};
use super::*;
fn resource() -> Resource {
    Resource {
        id: "config".into(),
        mime_type: "application/json".into(),
        size: 13,
        digest: "a".repeat(64),
        dimensions: [0, 0],
    }
}
fn package() -> Package {
    Package {
        protocol_version: package::PROTOCOL.into(),
        grant: grant(),
        device_public_key: BASE64.encode([19; 32]),
        resources: vec![resource()],
    }
}
fn signed_package(p: &Package, domain: &[u8]) -> Vec<u8> {
    let bytes = serde_json::to_vec(p).unwrap();
    serde_json::to_vec(&Envelope {
        algorithm: "ed25519".into(),
        key_id: "fixture-issuer".into(),
        signature: BASE64.encode(key().sign(&[domain, &bytes].concat()).as_ref()),
        payload: BASE64.encode(bytes),
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
fn standalone_package_is_not_an_accepted_current_offline_proof() {
    let p = package();
    let bytes = signed_package(&p, package::DOMAIN);
    assert_eq!(verify(&bytes).unwrap().resources, p.resources);
    assert!(accept(&bytes, &p.grant, SOURCE, 1001).is_err());
    for domain in [wire::DOMAIN, resume::DOMAIN, b"loom.surface.v1\0"] {
        assert!(verify(&signed_package(&p, domain)).is_err());
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
    payload["resources"] = serde_json::json!([]);
    envelope.payload = BASE64.encode(serde_json::to_vec(&payload).unwrap());
    assert!(verify(&serde_json::to_vec(&envelope).unwrap()).is_err());
}
#[test]
fn package_without_device_delegation_cannot_resume_even_with_valid_grant() {
    let mut p = package();
    p.device_public_key.clear();
    let bytes = signed_package(&p, package::DOMAIN);
    assert!(verify(&bytes).is_ok());
    let k = key();
    assert!(resume::Ticket::verify(
        &bytes,
        Issuer {
            key_id: "fixture-issuer",
            public_key: k.public_key().as_ref().try_into().unwrap()
        }
    )
    .is_err());
}
#[test]
fn package_resources_reject_alias_duplicate_dimensions_overflow_and_aggregate_budgets() {
    for change in 0..9 {
        let mut p = package();
        let r = &mut p.resources[0];
        match change {
            0 => r.id = "../config".into(),
            1 => r.size = 0,
            2 => r.size = 256 * 1024 + 1,
            3 => r.digest = "A".repeat(64),
            4 => r.dimensions = [1, 1],
            5 => r.mime_type = "text/html".into(),
            6 => {
                r.mime_type = "image/png".into();
                r.dimensions = [u32::MAX, u32::MAX]
            }
            7 => p.resources.push(resource()),
            _ => {
                p.resources = (0..17)
                    .map(|i| {
                        let mut r = resource();
                        r.id = format!("r{i}");
                        r
                    })
                    .collect()
            }
        }
        assert!(
            verify(&signed_package(&p, package::DOMAIN)).is_err(),
            "change {change}"
        );
    }
    let mut p = package();
    p.resources[0].size = 256 * 1024;
    p.resources.push(Resource {
        id: "other".into(),
        ..p.resources[0].clone()
    });
    assert!(verify(&signed_package(&p, package::DOMAIN)).is_err());
}
