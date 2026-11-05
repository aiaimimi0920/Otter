//! 独立 v3 reader 的方向/预算/签名与保存兼容回归；不以保存提示代授权。
use super::*;
use package::visibility as v3;
fn modern() -> v3::Package {
    let p = proof();
    v3::Package {
        protocol_version: v3::PROTOCOL.into(),
        package: p.package,
        ports: p.ports,
        visibility: v3::Visibility {
            inputs: vec![false],
            outputs: vec![],
        },
    }
}
fn read(bytes: &[u8]) -> Result<v3::Package, Error> {
    let k = key();
    v3::Package::verify(
        bytes,
        Issuer {
            key_id: "fixture-issuer",
            public_key: k.public_key().as_ref().try_into().unwrap(),
        },
    )
}
#[test]
fn visibility_proof_has_independent_domain_and_survives_offline_ticket() {
    let p = modern();
    let bytes = signed_proof(&p, v3::DOMAIN);
    assert_eq!(read(&bytes).unwrap().visibility, p.visibility);
    assert!(verify(&bytes).is_err());
    for domain in [DOMAIN, package::DOMAIN, resume::DOMAIN, wire::DOMAIN] {
        assert!(read(&signed_proof(&p, domain)).is_err());
    }
    let k = key();
    let ticket = resume::Ticket::verify(
        &bytes,
        Issuer {
            key_id: "fixture-issuer",
            public_key: k.public_key().as_ref().try_into().unwrap(),
        },
    )
    .unwrap();
    assert_eq!(ticket.ports, Some(p.ports.clone()));
    assert_eq!(ticket.visibility, Some(p.visibility));
    let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
    let mut payload: serde_json::Value =
        serde_json::from_slice(&BASE64.decode(&envelope.payload).unwrap()).unwrap();
    payload["visibility"]["inputs"][0] = true.into();
    envelope.payload = BASE64.encode(serde_json::to_vec(&payload).unwrap());
    assert!(read(&serde_json::to_vec(&envelope).unwrap()).is_err());
}
#[test]
fn visibility_reader_rejects_bad_shape_missing_unknown_non_bool_and_wrong_version() {
    for change in 0..8 {
        let mut p = serde_json::to_value(modern()).unwrap();
        match change {
            0 => {
                p.as_object_mut().unwrap().remove("visibility");
            }
            1 => p["visibility"]["inputs"] = serde_json::json!([]),
            2 => p["visibility"]["outputs"] = serde_json::json!([true]),
            3 => p["visibility"]["inputs"][0] = serde_json::json!("false"),
            4 => p["visibility"]["inputs"][0] = serde_json::Value::Null,
            5 => p["visibility"]["unknown"] = true.into(),
            6 => p["protocolVersion"] = PROTOCOL.into(),
            _ => p["ports"]["inputs"][0]["defaultVisible"] = true.into(),
        }
        assert!(
            read(&signed_proof(&p, v3::DOMAIN)).is_err(),
            "change {change}"
        );
    }
    let mut empty = modern();
    empty.ports.inputs.clear();
    empty.visibility.inputs.clear();
    assert!(read(&signed_proof(&empty, v3::DOMAIN)).is_ok());
}
#[test]
fn visibility_record_preserves_direction_and_legacy_record_defaults_to_visible() {
    let mut p = modern();
    p.ports.outputs = p.ports.inputs.clone();
    p.visibility.outputs = vec![true];
    let mut record = Record::new(&p.package.grant.binding, p.ports);
    assert!(record.validate("vendor/demo").is_ok());
    assert!(!serde_json::to_value(&record)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("visibility"));
    record.visibility = Some(p.visibility);
    let restored: Record = serde_json::from_slice(&serde_json::to_vec(&record).unwrap()).unwrap();
    assert!(restored.validate("vendor/demo").is_ok());
    let visibility = restored.visibility.as_ref().unwrap();
    assert!(!visibility.visible(&restored.ports, "原始图片", true));
    assert!(visibility.visible(&restored.ports, "原始图片", false));
    assert!(visibility.visible(&restored.ports, "undeclared-result", false));
    record.visibility.as_mut().unwrap().outputs.clear();
    assert!(record.validate("vendor/demo").is_err());
}
#[test]
fn visibility_shares_the_existing_port_byte_budget() {
    let mut p = modern();
    p.ports.inputs.clear();
    p.visibility.inputs.clear();
    // Find a valid old port declaration immediately below its byte gate.
    for i in 0..64 {
        let ports = if i < 32 {
            &mut p.ports.inputs
        } else {
            &mut p.ports.outputs
        };
        let flags = if i < 32 {
            &mut p.visibility.inputs
        } else {
            &mut p.visibility.outputs
        };
        ports.push(Port {
            name: format!("{i:02}{}", "a".repeat(126)),
            kind: "b".repeat(64),
        });
        flags.push(false);
        if p.ports.validate().is_ok() && p.visibility.validate(&p.ports).is_err() {
            return;
        }
    }
    panic!("fixture did not exercise shared budget");
}
