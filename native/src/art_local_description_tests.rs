//! 描述前登记的代际与信任必须与下载相同；测试只验证 owner，不冒充网络或窗口挂载。
use super::*;
use crate::art_local_requests::tests as fixture;
fn request() -> Request {
    Request {
        origin: fixture::request().binding.origin,
        workspace: "workspace:1".into(),
        instance: "instance:1".into(),
        art: "vendor/demo".into(),
    }
}
#[test]
fn description_replacement_revokes_original_lease_and_uses_registry_generation() {
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let mut registry = Registry::new("workspace:1".into());
    let old = fixture::lease(&mut owner);
    let first = registry
        .begin_description(fixture::id(), request(), old.clone(), &mut trust)
        .unwrap();
    let current = fixture::lease(&mut owner);
    let second = registry
        .begin_description(fixture::id(), request(), current.clone(), &mut trust)
        .unwrap();
    assert!(old.is_revoked());
    assert!(!current.is_revoked());
    assert_eq!(second.token.generation, first.token.generation + 1);
    let failure = Completion::failed(first.unit, first.token, Error::Worker);
    assert!(matches!(registry.accept(failure), Err(Error::Stale)));
    assert!(!current.is_revoked());
    let failure = Completion::failed(second.unit, second.token, Error::Worker);
    assert!(matches!(registry.accept(failure), Err(Error::Worker)));
    assert!(current.is_revoked());
}
#[test]
fn description_is_trust_registered_before_any_network_and_rejects_reused_lease() {
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let mut registry = Registry::new("workspace:1".into());
    let lease = fixture::lease(&mut owner);
    let pending = registry
        .begin_description(fixture::id(), request(), lease.clone(), &mut trust)
        .unwrap();
    assert!(matches!(
        registry.begin(fixture::id(), fixture::request(), lease.clone(), &mut trust),
        Err(Error::Lease)
    ));
    trust.replace(Default::default()).unwrap();
    assert!(lease.is_revoked());
    assert_eq!(
        registry.claim(&pending.unit, &pending.token),
        Err(Error::Stale)
    );
}
#[test]
fn invalid_description_context_does_not_replace_current_owner() {
    let mut owner = fixture::owner();
    let mut trust = fixture::authority();
    let mut registry = Registry::new("workspace:1".into());
    let lease = fixture::lease(&mut owner);
    let first = registry
        .begin(fixture::id(), fixture::request(), lease.clone(), &mut trust)
        .unwrap();
    for kind in 0..3 {
        let mut input = request();
        match kind {
            0 => input.workspace = "other".into(),
            1 => input.art = "x".repeat(258),
            _ => input.instance.clear(),
        }
        assert!(registry
            .begin_description(fixture::id(), input, fixture::lease(&mut owner), &mut trust)
            .is_err());
        assert!(!lease.is_revoked());
    }
    assert!(registry.claim(&first.unit, &first.token).is_ok());
}
#[test]
fn removing_description_node_and_clearing_workspace_rejects_late_failure() {
    for clear in [false, true] {
        let mut owner = fixture::owner();
        let mut trust = fixture::authority();
        let mut registry = Registry::new("workspace:1".into());
        let lease = fixture::lease(&mut owner);
        let pending = registry
            .begin_description(fixture::id(), request(), lease.clone(), &mut trust)
            .unwrap();
        if clear {
            registry.clear();
        } else {
            registry.retain_units(|_| false);
        }
        assert!(lease.is_revoked());
        assert!(matches!(
            registry.accept(Completion::failed(
                pending.unit,
                pending.token,
                Error::Worker
            )),
            Err(Error::Stale)
        ));
    }
}
