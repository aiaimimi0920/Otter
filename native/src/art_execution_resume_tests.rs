//! 独立域、精确稳定身份、新宿主绑定、原期限和不可逆停止，沿既有固定签名fixture。
use super::*;
use crate::art_execution_grant::{
    package::visibility::{DOMAIN, PROTOCOL},
    resume::Ticket,
};
fn ticket() -> Ticket {
    Ticket {
        protocol_version: PROTOCOL.into(),
        device_public_key: BASE64.encode([19; 32]),
        grant: grant(),
        resources: Vec::new(),
        ports: None,
        visibility: None,
    }
}
fn signed_resume(value: &Ticket, domain: &[u8]) -> Vec<u8> {
    let proof = package::visibility::Package {
        protocol_version: value.protocol_version.clone(),
        package: package::Package {
            protocol_version: package::PROTOCOL.into(),
            device_public_key: value.device_public_key.clone(),
            grant: value.grant.clone(),
            resources: value.resources.clone(),
        },
        ports: package::ports::Ports {
            inputs: vec![],
            outputs: vec![],
        },
        visibility: package::visibility::Visibility {
            inputs: vec![],
            outputs: vec![],
        },
    };
    let payload = serde_json::to_vec(&proof).unwrap();
    serde_json::to_vec(&Envelope {
        algorithm: "ed25519".into(),
        key_id: "fixture-issuer".into(),
        signature: BASE64.encode(key().sign(&[domain, payload.as_slice()].concat()).as_ref()),
        payload: BASE64.encode(payload),
    })
    .unwrap()
}
fn resumed(
    bytes: &[u8],
    binding: &Binding,
    permissions: &[Permission],
    services: &[String],
    source: &[u8],
    now: u64,
) -> Result<Permit, Error> {
    let key = key();
    Permit::accept_resumed(
        bytes,
        Issuer {
            key_id: "fixture-issuer",
            public_key: key.public_key().as_ref().try_into().unwrap(),
        },
        Expected {
            binding,
            permissions,
            services,
        },
        source,
        now,
    )
}
#[test]
fn resume_requires_separate_domain_and_never_reinterprets_a_v1_grant() {
    let value = ticket();
    for bytes in [
        signed(&value.grant),
        signed_resume(&value, wire::DOMAIN),
        signed_resume(&value, b"loom.surface.v1\0"),
    ] {
        assert!(resumed(
            &bytes,
            &value.grant.binding,
            &value.grant.permissions,
            &value.grant.services,
            SOURCE,
            1001
        )
        .is_err());
    }
    let bytes = signed_resume(&value, DOMAIN);
    assert!(accept(&bytes, &value.grant, SOURCE, 1001).is_err());
}
#[test]
fn resume_changes_only_host_request_identity_and_cannot_extend_offline_time() {
    let value = ticket();
    let mut current = value.grant.binding.clone();
    current.request_id = "request:new".into();
    current.workspace_id = "workspace:new".into();
    current.instance_id = "instance:new".into();
    current.generation = 7;
    let mut permit = resumed(
        &signed_resume(&value, DOMAIN),
        &current,
        &value.grant.permissions,
        &value.grant.services,
        SOURCE,
        2000,
    )
    .unwrap();
    assert_eq!(permit.deadline(false).0, 5000);
    assert_eq!(permit.deadline(true).0, 5000);
    assert_eq!(
        permit
            .authorize(&current, 2001, false, false)
            .unwrap()
            .binding,
        current
    );
    assert_eq!(
        permit.authorize(&current, 5000, false, false).err(),
        Some(Error::Time)
    );
    assert_eq!(
        permit.authorize(&current, 2002, false, false).err(),
        Some(Error::Stopped)
    );
}
#[test]
fn resume_rejects_changed_origin_device_package_sdk_scope_and_source() {
    let value = ticket();
    let bytes = signed_resume(&value, DOMAIN);
    for change in 0..6 {
        let mut current = value.grant.binding.clone();
        match change {
            0 => current.origin = "https://other.example".into(),
            1 => current.device_id = "device:other".into(),
            2 => current.art_id = "vendor/other".into(),
            3 => current.art_version = "2.0.0".into(),
            4 => current.package_digest = "b".repeat(64),
            _ => current.sdk_version = "otter.art.v2".into(),
        }
        assert!(resumed(
            &bytes,
            &current,
            &value.grant.permissions,
            &value.grant.services,
            SOURCE,
            1001
        )
        .is_err());
    }
    assert!(resumed(
        &bytes,
        &value.grant.binding,
        &[Permission::LocalState, Permission::Preview],
        &[],
        SOURCE,
        1001
    )
    .is_err());
    assert!(resumed(
        &bytes,
        &value.grant.binding,
        &value.grant.permissions,
        &value.grant.services,
        b"changed",
        1001
    )
    .is_err());
}
#[test]
fn resume_rejects_disabled_expired_future_malformed_and_tampered_tickets() {
    for change in 0..5 {
        let mut value = ticket();
        match change {
            0 => value.grant.offline_until_ms = 0,
            1 => value.grant.offline_until_ms = 1001,
            2 => value.grant.issued_at_ms = 2000,
            3 => value.device_public_key = "bad".into(),
            _ => value.protocol_version = "loom.art.local.grant.v1".into(),
        }
        assert!(resumed(
            &signed_resume(&value, DOMAIN),
            &value.grant.binding,
            &value.grant.permissions,
            &value.grant.services,
            SOURCE,
            1001
        )
        .is_err());
    }
    let value = ticket();
    let mut bytes = signed_resume(&value, DOMAIN);
    let i = bytes.len() / 2;
    bytes[i] ^= 1;
    assert!(resumed(
        &bytes,
        &value.grant.binding,
        &value.grant.permissions,
        &value.grant.services,
        SOURCE,
        1001
    )
    .is_err());
}
#[test]
fn local_lease_requires_admission_has_no_network_credential_and_stops_irreversibly() {
    let value = ticket();
    let lease =
        crate::device_authorization_lease::Lease::pending_local(&value.grant.binding.origin);
    // 不在待验证租约上调用validate试探：失败会不可逆撤销，后台只能一次显式激活。
    assert_eq!(lease.expires_at_ms(), 0);
    assert_eq!(lease.device_id(), "");
    lease.accept_local(&value, 1001).unwrap();
    assert_eq!(lease.device_id(), value.grant.binding.device_id);
    assert_eq!(lease.expires_at_ms(), 5000);
    assert!(lease.validate(&value.grant.binding.origin, 1002).is_ok());
    let (_, client, _, _) = crate::device_session_client::tests::fixture_with_pem();
    assert!(lease.control(&client).is_err());
    assert!(lease.execution_control(&client).is_err());
    assert!(lease.validate(&value.grant.binding.origin, 1001).is_err());
    assert!(lease.is_revoked());
    assert!(lease.validate(&value.grant.binding.origin, 1003).is_err());
}
