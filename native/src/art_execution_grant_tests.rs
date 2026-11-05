//! 两实现独立验签、精确身份、源码完整性及不可逆失效；不冒充真实挂载/服务执行。
use super::*;
use ring::signature::{Ed25519KeyPair, KeyPair};
use wire::{Source, PROTOCOL, SDK};
const SOURCE: &[u8] = b"globalThis.art = { preview: 'local' };";
#[path = "art_execution_package_tests.rs"]
mod package_tests;
#[path = "art_execution_ports_tests.rs"]
mod ports_tests;
#[path = "art_execution_resume_tests.rs"]
mod resume_tests;
#[test]
fn online_host_observation_never_advances_worker_clock_or_revives_stopped_permit() {
    let value = grant();
    let mut permit = accept(&signed(&value), &value, SOURCE, 1001).unwrap();
    assert_eq!(permit.validate_online_host(&value.binding, 1003), Ok(()));
    assert_eq!(permit.observed_wall(), 1001);
    // A service that sampled before UI observation remains valid; UI must not mutate its clock.
    assert!(permit.authorize(&value.binding, 1002, true, false).is_ok());
    assert_eq!(
        permit.validate_online_host(&value.binding, 1001),
        Err(Error::Time)
    );
    assert_eq!(permit.observed_wall(), 1002);
    assert_eq!(
        permit.validate_online_host(&value.binding, 10000),
        Err(Error::Time)
    );
    let mut changed = value.binding.clone();
    changed.generation += 1;
    assert_eq!(
        permit.validate_online_host(&changed, 1003),
        Err(Error::Binding)
    );
    permit.stop();
    assert_eq!(
        permit.validate_online_host(&value.binding, 1003),
        Err(Error::Stopped)
    );
}
#[test]
fn cancellation_only_preserves_stop_binding_and_original_time_bounds() {
    let value = grant();
    let mut permit = accept(&signed(&value), &value, SOURCE, 1001).unwrap();
    let deadline = permit.deadline(true);
    permit.stop();
    assert_eq!(permit.authorize_cancel(&value.binding, 1002), Ok(()));
    assert_eq!(permit.deadline(true), deadline);
    assert_eq!(
        permit.authorize(&value.binding, 1002, true, false).err(),
        Some(Error::Stopped)
    );
    let mut changed = value.binding.clone();
    changed.generation += 1;
    assert_eq!(permit.authorize_cancel(&changed, 1003), Err(Error::Binding));
    assert_eq!(
        permit.authorize_cancel(&value.binding, 1001),
        Err(Error::Time)
    );
    assert_eq!(
        permit.authorize_cancel(&value.binding, value.expires_at_ms),
        Err(Error::Time)
    );
    permit.started = Instant::now() - permit.remaining;
    assert_eq!(
        permit.authorize_cancel(&value.binding, 1003),
        Err(Error::Time)
    );
}
#[test]
fn wait_deadline_preserves_original_admission_and_never_extends_offline_scope() {
    let value = grant();
    let permit = accept(&signed(&value), &value, SOURCE, 1001).unwrap();
    let online = permit.deadline(true);
    let offline = permit.deadline(false);
    assert_eq!(online.0, 10000);
    assert_eq!(online.1, permit.started + Duration::from_millis(8999));
    assert_eq!(offline.0, 5000);
    assert_eq!(offline.1, permit.started + Duration::from_millis(3999));
    assert!(offline.1 < online.1);
    assert_eq!(permit.deadline(true), online);
    let mut denied = value;
    denied.offline_until_ms = 0;
    let permit = accept(&signed(&denied), &denied, SOURCE, 1001).unwrap();
    assert_eq!(permit.deadline(false), (0, permit.started));
}
fn grant() -> Grant {
    let digest = format!("{:x}", Sha256::digest(SOURCE));
    Grant {
        protocol_version: PROTOCOL.into(),
        grant_id: "grant:fixture".into(),
        binding: Binding {
            origin: "https://loom.example:9443".into(),
            request_id: "request:1".into(),
            device_id: "device:1".into(),
            workspace_id: "workspace:1".into(),
            instance_id: "instance:1".into(),
            generation: 1,
            art_id: "vendor/demo".into(),
            art_version: "1.2.3".into(),
            package_digest: "a".repeat(64),
            sdk_version: SDK.into(),
        },
        execution_owner: "otter".into(),
        result_owner: "otter_workspace".into(),
        permissions: vec![
            Permission::LocalState,
            Permission::Preview,
            Permission::LoomService,
        ],
        services: vec!["service:echo".into()],
        issued_at_ms: 1000,
        expires_at_ms: 10000,
        offline_until_ms: 5000,
        source: Source {
            resource_id: format!("sha256:{digest}"),
            digest,
            size: SOURCE.len() as u64,
        },
    }
}
fn key() -> Ed25519KeyPair {
    Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap()
}
fn signed(value: &Grant) -> Vec<u8> {
    let payload = serde_json::to_vec(value).unwrap();
    let signature = key().sign(&[wire::DOMAIN, payload.as_slice()].concat());
    serde_json::to_vec(&Envelope {
        algorithm: "ed25519".into(),
        key_id: "fixture-issuer".into(),
        payload: BASE64.encode(payload),
        signature: BASE64.encode(signature.as_ref()),
    })
    .unwrap()
}
fn accept(bytes: &[u8], expected: &Grant, source: &[u8], now: u64) -> Result<Permit, Error> {
    let key = key();
    let pin: &[u8; 32] = key.public_key().as_ref().try_into().unwrap();
    Permit::accept(
        bytes,
        Issuer {
            key_id: "fixture-issuer",
            public_key: pin,
        },
        Expected {
            binding: &expected.binding,
            permissions: &expected.permissions,
            services: &expected.services,
        },
        source,
        now,
    )
}
#[test]
fn signed_source_and_exact_scope_are_required_before_permit_exists() {
    let value = grant();
    let bytes = signed(&value);
    assert!(accept(&bytes, &value, SOURCE, 1001).is_ok());
    assert!(matches!(
        accept(&bytes, &value, b"changed", 1001),
        Err(Error::Source)
    ));
    let changes: &[fn(&mut Binding)] = &[
        |b| b.origin = "https://other.example".into(),
        |b| b.request_id = "request:2".into(),
        |b| b.device_id = "device:2".into(),
        |b| b.workspace_id = "workspace:2".into(),
        |b| b.instance_id = "instance:2".into(),
        |b| b.generation = 2,
        |b| b.art_id = "vendor/other".into(),
        |b| b.art_version = "1.2.4".into(),
        |b| b.package_digest = "b".repeat(64),
        |b| b.sdk_version = "otter.art.v2".into(),
    ];
    for change in changes {
        let mut expected = value.clone();
        change(&mut expected.binding);
        assert!(matches!(
            accept(&bytes, &expected, SOURCE, 1001),
            Err(Error::Binding)
        ));
    }
    let mut expected = value.clone();
    expected.services = vec!["service:admin".into()];
    assert!(matches!(
        accept(&bytes, &expected, SOURCE, 1001),
        Err(Error::Scope)
    ));
}
#[test]
fn altered_payload_wrong_key_and_legacy_tokens_never_authorize_execution() {
    let value = grant();
    let bytes = signed(&value);
    let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
    let mut changed = value.clone();
    changed.binding.generation = 2;
    envelope.payload = BASE64.encode(serde_json::to_vec(&changed).unwrap());
    assert!(matches!(
        accept(
            &serde_json::to_vec(&envelope).unwrap(),
            &changed,
            SOURCE,
            1001
        ),
        Err(Error::Signature)
    ));
    envelope = serde_json::from_slice(&bytes).unwrap();
    envelope.key_id = "untrusted".into();
    assert!(matches!(
        accept(
            &serde_json::to_vec(&envelope).unwrap(),
            &value,
            SOURCE,
            1001
        ),
        Err(Error::Signature)
    ));
    for bytes in [
        b"{\"token\":\"device-token\"}".as_slice(),
        b"{\"leaseId\":\"lease:1\"}",
    ] {
        assert!(matches!(
            accept(bytes, &value, SOURCE, 1001),
            Err(Error::Wire)
        ));
    }
    assert!(matches!(
        accept(&vec![b' '; wire::MAX_ENVELOPE + 1], &value, SOURCE, 1001),
        Err(Error::Limit)
    ));
}
#[test]
fn expiry_offline_deadline_revocation_and_rebinding_are_terminal() {
    let value = grant();
    let bytes = signed(&value);
    for (now, online, revoked, elapsed, error) in [
        (999, true, false, 0, Error::Time),
        (10000, true, false, 0, Error::Time),
        (5000, false, false, 0, Error::Time),
        (1001, false, false, 3999, Error::Time),
        (1001, true, false, 8999, Error::Time),
        (1001, true, true, 0, Error::Stopped),
    ] {
        let mut permit = accept(&bytes, &value, SOURCE, 1001).unwrap();
        assert_eq!(
            permit.check(
                &value.binding,
                now,
                online,
                revoked,
                Duration::from_millis(elapsed)
            ),
            Err(error)
        );
        assert_eq!(
            permit.check(&value.binding, 1002, true, false, Duration::ZERO),
            Err(Error::Stopped)
        );
    }
    let mut permit = accept(&bytes, &value, SOURCE, 1001).unwrap();
    assert!(permit.authorize(&value.binding, 1002, false, false).is_ok());
    let mut next = value.binding.clone();
    next.generation += 1;
    assert_eq!(
        permit.check(&next, 1003, true, false, Duration::ZERO),
        Err(Error::Binding)
    );
    assert!(permit.authorize(&value.binding, 1003, true, false).is_err());
}
#[test]
fn signed_but_invalid_contract_is_rejected_and_stop_cannot_be_undone() {
    let changes: &[fn(&mut Grant)] = &[
        |g| g.protocol_version = "loom.surface.v1".into(),
        |g| g.execution_owner = "loom".into(),
        |g| g.permissions.push(Permission::Preview),
        |g| g.services.push("*".into()),
        |g| g.offline_until_ms = 10001,
        |g| g.expires_at_ms = 1_000_000,
    ];
    for change in changes {
        let mut value = grant();
        change(&mut value);
        assert!(accept(&signed(&value), &value, SOURCE, 1001).is_err());
    }
    let value = grant();
    let mut permit = accept(&signed(&value), &value, SOURCE, 1001).unwrap();
    permit.stop();
    assert!(permit.authorize(&value.binding, 1002, true, false).is_err());
}

#[test]
fn installed_package_identity_uses_publisher_and_art_without_path_aliases() {
    for id in [
        "vendor:demo",
        "vendor/../demo",
        "vendor/demo/extra",
        "vendor/CON",
        "vendor/COM01",
        "/demo",
        "vendor/.demo",
    ] {
        let mut value = grant();
        value.binding.art_id = id.into();
        assert!(
            matches!(
                accept(&signed(&value), &value, SOURCE, 1001),
                Err(Error::Binding)
            ),
            "{id}"
        );
    }
    let value = grant();
    assert!(accept(&signed(&value), &value, SOURCE, 1001).is_ok());
}
#[test]
#[ignore = "requires the current Loom producer artifact via OTTER_LOCAL_ART_FIXTURE"]
fn consumes_actual_loom_signed_grant() {
    let file = std::env::var_os("OTTER_LOCAL_ART_FIXTURE").expect("Loom fixture required");
    let bytes = std::fs::read(file).unwrap();
    let expected = grant();
    let mut permit = accept(&bytes, &expected, SOURCE, 1001).unwrap();
    assert_eq!(
        permit
            .authorize(&expected.binding, 1002, false, false)
            .unwrap(),
        &expected
    );
    permit.stop();
    assert!(permit
        .authorize(&expected.binding, 1003, true, false)
        .is_err());
}

#[test]
fn same_key_id_cannot_substitute_public_key_or_signature_domain() {
    let value = grant();
    let bytes = signed(&value);
    let wrong = Ed25519KeyPair::from_seed_unchecked(&[8; 32]).unwrap();
    let pin = wrong.public_key().as_ref().try_into().unwrap();
    assert!(matches!(
        Permit::accept(
            &bytes,
            Issuer {
                key_id: "fixture-issuer",
                public_key: pin
            },
            Expected {
                binding: &value.binding,
                permissions: &value.permissions,
                services: &value.services
            },
            SOURCE,
            1001
        ),
        Err(Error::Signature)
    ));
    let mut envelope: Envelope = serde_json::from_slice(&bytes).unwrap();
    envelope.signature = BASE64.encode(
        key()
            .sign(&BASE64.decode(&envelope.payload).unwrap())
            .as_ref(),
    );
    assert!(matches!(
        accept(
            &serde_json::to_vec(&envelope).unwrap(),
            &value,
            SOURCE,
            1001
        ),
        Err(Error::Signature)
    ));
}

#[test]
fn even_a_trusted_signature_does_not_allow_duplicate_or_unknown_fields() {
    let value = grant();
    let original = serde_json::to_string(&value).unwrap();
    for extra in [
        "\"publicKey\":\"untrusted\",",
        "\"protocolVersion\":\"loom.surface.v1\",",
    ] {
        let payload = format!("{{{extra}{}", &original[1..]);
        let envelope = Envelope {
            algorithm: "ed25519".into(),
            key_id: "fixture-issuer".into(),
            signature: BASE64.encode(
                key()
                    .sign(&[wire::DOMAIN, payload.as_bytes()].concat())
                    .as_ref(),
            ),
            payload: BASE64.encode(&payload),
        };
        assert!(matches!(
            accept(
                &serde_json::to_vec(&envelope).unwrap(),
                &value,
                SOURCE,
                1001
            ),
            Err(Error::Wire)
        ));
    }
    let mut outer: serde_json::Value = serde_json::from_slice(&signed(&value)).unwrap();
    outer["publicKey"] = serde_json::json!("peer key");
    assert!(matches!(
        accept(&serde_json::to_vec(&outer).unwrap(), &value, SOURCE, 1001),
        Err(Error::Wire)
    ));
}
