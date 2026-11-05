//! 请求代际、一次交付、失败归属与原租约取消；不创建另一套请求代际实现。
use super::*;
use crate::{
    art_execution_grant::wire,
    art_issuer_trust::{Document, Entry},
    device_authorization_lease::Grant,
    device_identity::Identity,
    device_session_attempt::Attempt,
};
use base64::{engine::general_purpose::STANDARD, Engine};
const ORIGIN: &str = "https://loom.example";
pub(crate) fn id() -> UnitId {
    UnitId::parse("node:1").unwrap()
}
pub(crate) fn request() -> Request {
    Request {
        binding: wire::Binding {
            origin: ORIGIN.into(),
            request_id: "script-supplied".into(),
            device_id: "device:1".into(),
            workspace_id: "workspace:1".into(),
            instance_id: "instance:1".into(),
            generation: 1,
            art_id: "vendor/demo".into(),
            art_version: "1.0.0".into(),
            package_digest: "a".repeat(64),
            sdk_version: wire::SDK.into(),
        },
        permissions: vec![wire::Permission::LocalState, wire::Permission::Preview],
        services: vec![],
    }
}
pub(crate) fn authority() -> Authority {
    Authority::new(Document {
        issuers: vec![Entry {
            origin: ORIGIN.into(),
            key_id: "issuer".into(),
            public_key: STANDARD.encode([7; 32]),
        }],
    })
    .unwrap()
}
pub(crate) fn owner() -> Grant {
    let mut identity = Identity::generate().unwrap();
    identity.bind("device:1").unwrap();
    Grant::new(
        Attempt::new(&identity, ORIGIN)
            .unwrap()
            .challenge(
                200,
                br#"{"deviceId":"device:1","challengeId":"id","challenge":"value"}"#,
            )
            .unwrap()
            .finish(
                200,
                br#"{"deviceId":"device:1","token":"test-only","expiresAtMs":9007199254740991}"#,
                1,
            )
            .unwrap(),
    )
}
pub(crate) fn lease(owner: &mut Grant) -> Arc<Lease> {
    Arc::new(owner.lease(ORIGIN, 1).unwrap())
}
#[test]
fn new_attempt_replaces_request_id_and_revokes_old_chain() {
    let mut owner = owner();
    let mut authority = authority();
    let mut registry = Registry::new("workspace:1".into());
    let old = lease(&mut owner);
    let first = registry
        .begin(id(), request(), old.clone(), &mut authority)
        .unwrap();
    assert_ne!(first.request.binding.request_id, "script-supplied");
    assert!(first
        .request
        .binding
        .request_id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b)));
    let current = lease(&mut owner);
    let second = registry
        .begin(id(), request(), current.clone(), &mut authority)
        .unwrap();
    assert!(old.is_revoked());
    assert!(!current.is_revoked());
    assert_ne!(first.token, second.token);
    assert_eq!(registry.claim(&id(), &first.token), Err(Error::Stale));
    assert!(registry.claim(&id(), &second.token).is_ok());
    assert_eq!(registry.claim(&id(), &second.token), Err(Error::Stale));
    registry.cancel(&id());
    assert!(current.is_revoked());
}
#[test]
fn late_failure_cannot_cancel_new_request_but_current_failure_does() {
    let mut owner = owner();
    let mut authority = authority();
    let mut registry = Registry::new("workspace:1".into());
    let first = registry
        .begin(id(), request(), lease(&mut owner), &mut authority)
        .unwrap();
    let current = lease(&mut owner);
    let second = registry
        .begin(id(), request(), current.clone(), &mut authority)
        .unwrap();
    let failure = |p: Pending| Completion {
        unit: p.unit,
        token: p.token,
        result: Err(Error::Download(crate::art_issuer_trust::Error::Trust)),
    };
    assert!(matches!(registry.accept(failure(first)), Err(Error::Stale)));
    assert!(!current.is_revoked());
    assert!(matches!(
        registry.accept(failure(second)),
        Err(Error::Download(_))
    ));
    assert!(current.is_revoked());
}
#[test]
fn bad_workspace_or_reused_lease_does_not_destroy_current_request() {
    let mut owner = owner();
    let mut authority = authority();
    let mut registry = Registry::new("workspace:1".into());
    let current = lease(&mut owner);
    let first = registry
        .begin(id(), request(), current.clone(), &mut authority)
        .unwrap();
    assert!(matches!(
        registry.begin(id(), request(), current.clone(), &mut authority),
        Err(Error::Lease)
    ));
    let mut invalid = request();
    invalid.binding.workspace_id = "workspace:other".into();
    assert!(matches!(
        registry.begin(id(), invalid, lease(&mut owner), &mut authority),
        Err(Error::Workspace)
    ));
    assert!(!current.is_revoked());
    assert!(registry.claim(&id(), &first.token).is_ok());
}
#[test]
fn clear_and_drop_invalidate_workspaces_without_reviving_old_tokens() {
    let mut owner = owner();
    let mut authority = authority();
    let mut registry = Registry::new("workspace:1".into());
    let old = lease(&mut owner);
    let first = registry
        .begin(id(), request(), old.clone(), &mut authority)
        .unwrap();
    registry.clear();
    assert!(old.is_revoked());
    let current = lease(&mut owner);
    let _second = registry
        .begin(id(), request(), current.clone(), &mut authority)
        .unwrap();
    assert_eq!(registry.claim(&id(), &first.token), Err(Error::Stale));
    drop(registry);
    assert!(current.is_revoked());
}

#[test]
fn retaining_nodes_revokes_only_removed_chain_and_releases_generation_budget() {
    let mut owner = owner();
    let mut authority = authority();
    let mut registry = Registry::new("workspace:1".into());
    let kept = lease(&mut owner);
    let pending = registry
        .begin(id(), request(), kept.clone(), &mut authority)
        .unwrap();
    for index in 0..crate::workspace::MAX_UNITS + 1 {
        let unit = UnitId::parse(&format!("removed:{index}")).unwrap();
        let removed = lease(&mut owner);
        let old = registry
            .begin(unit.clone(), request(), removed.clone(), &mut authority)
            .unwrap();
        registry.retain_units(|unit| unit == &id());
        assert!(removed.is_revoked());
        assert!(!kept.is_revoked());
        assert_eq!(registry.claim(&unit, &old.token), Err(Error::Stale));
    }
    assert!(registry.claim(&id(), &pending.token).is_ok());
}

#[test]
fn runtime_retirement_stops_execution_without_revoking_cleanup_authority() {
    for mode in 0..4 {
        let mut owner = owner();
        let mut authority = authority();
        let mut registry = Registry::new("workspace:1".into());
        let current = lease(&mut owner);
        let pending = registry
            .begin(id(), request(), current.clone(), &mut authority)
            .unwrap();
        registry.claim(&id(), &pending.token).unwrap();
        registry.mark_running(&id(), &pending.token);
        match mode {
            0 => registry.cancel(&id()),
            1 => registry.clear(),
            2 => registry.retain_units(|_| false),
            _ => {
                registry
                    .begin(id(), request(), lease(&mut owner), &mut authority)
                    .unwrap();
            }
        }
        assert!(current.execution_stopped());
        assert!(!current.is_revoked());
        assert_eq!(registry.claim(&id(), &pending.token), Err(Error::Stale));
        current.revoke(); // 模拟 Runtime 退出时的最终认证撤销。
    }
}

#[test]
fn delivery_without_runtime_and_stale_handoff_cannot_keep_authority() {
    let mut owner = owner();
    let mut authority = authority();
    let mut registry = Registry::new("workspace:1".into());
    let old = lease(&mut owner);
    let pending = registry
        .begin(id(), request(), old.clone(), &mut authority)
        .unwrap();
    registry.claim(&id(), &pending.token).unwrap();
    let current = lease(&mut owner);
    let next = registry
        .begin(id(), request(), current.clone(), &mut authority)
        .unwrap();
    assert!(old.is_revoked());
    registry.claim(&id(), &next.token).unwrap();
    registry.mark_running(&id(), &pending.token);
    registry.cancel(&id());
    assert!(current.is_revoked());
}
