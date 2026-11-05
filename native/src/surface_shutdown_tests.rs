//! 隐藏窗口与已撤权后台作业验证退出结算；不访问真实服务或用户配置。
use super::*;
use crate::{
    desktop::{composition::relay::sessions::Sessions, surface_windows::Windows},
    surface_lifecycle_owner::Owner,
    surface_live_binding::Lifecycle,
};

fn window(grant: &mut crate::device_authorization_lease::Grant) -> AuthorizedWindow {
    AuthorizedWindow::new(
        &outcome(),
        grant.lease("https://localhost:1234", 0).unwrap(),
        0,
        [200, 140],
    )
    .unwrap()
}

fn acknowledge(owner: &mut Owner, state: Lifecycle, label: &str) {
    let revision = owner.begin(state).unwrap();
    let binding = owner.binding();
    let bytes = serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(), "attachmentId":binding.attachment(),
        "hookNodeId":binding.node(), "deviceId":binding.device()},
        "lifecycle":label, "lifecycleRevision":revision}))
    .unwrap();
    owner.acknowledge(&bytes, state, revision).unwrap();
}

fn completed(owner: Owner) -> Sessions {
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    drop(grant);
    let config = crate::device_authorization_config::Config::new(
        "https://localhost:1234",
        std::path::Path::new(
            "C:/Users/Public/nas_home/AI/GameEditor/linshi/unused-surface-shutdown",
        ),
        "test",
    )
    .unwrap();
    let mut remote = Sessions::default();
    remote
        .start_surface(
            config,
            lease,
            owner,
            crate::surface_lifecycle_runtime::Action::Recover,
        )
        .unwrap_or_else(|error| panic!("{}", error.error));
    assert!(remote.surface_cleanup_pending());
    remote.close();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !remote.finished() {
        assert!(std::time::Instant::now() < deadline);
        assert!(remote.poll_surfaces().is_empty());
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(remote.surface_cleanup_pending());
    remote
}

#[test]
fn shutdown_reconciles_late_inactive_ack_without_reopening_or_losing_owner() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = window(&mut grant);
    window.close().unwrap();
    let mut owner = window.begin_deactivation(0).unwrap().unwrap();
    acknowledge(&mut owner, Lifecycle::Inactive, "inactive");
    let binding = owner.binding().clone();
    let mut remote = completed(owner);
    let mut windows = Windows::default();
    assert!(windows.insert(window).is_ok());
    drop(grant);
    windows.settle_shutdown(&mut remote, u64::MAX).unwrap();
    assert!(!remote.surface_cleanup_pending());
    windows.settle_shutdown(&mut remote, u64::MAX).unwrap();
    let closed = windows.take_closed().unwrap();
    assert!(!closed.is_shown());
    assert_eq!(closed.lifecycle().unwrap().binding(), &binding);
    assert!(!closed.lifecycle().unwrap().disposed());
}

#[test]
fn shutdown_late_active_ack_is_retained_but_not_reported_as_clean() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = window(&mut grant);
    let mut owner = window.begin_activation(0).unwrap().unwrap();
    acknowledge(&mut owner, Lifecycle::Active, "active");
    let mut remote = completed(owner);
    let mut windows = Windows::default();
    assert!(windows.insert(window).is_ok());
    assert!(windows.settle_shutdown(&mut remote, 0).is_err());
    assert!(!remote.surface_cleanup_pending());
    let closed = windows.take_closed().unwrap();
    assert!(!closed.is_alive());
    assert!(!closed.is_shown());
    assert!(closed.lifecycle().unwrap().confirmed_active());
}

#[test]
fn shutdown_unknown_operation_returns_owner_without_replay() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut window = window(&mut grant);
    window.close().unwrap();
    let mut owner = window.begin_deactivation(0).unwrap().unwrap();
    owner.begin(Lifecycle::Inactive).unwrap();
    let mut remote = completed(owner);
    let mut windows = Windows::default();
    assert!(windows.insert(window).is_ok());
    assert!(windows.settle_shutdown(&mut remote, 0).is_err());
    assert!(!remote.surface_cleanup_pending());
    let mut closed = windows.take_closed().unwrap();
    assert!(closed.lifecycle().unwrap().uncertain());
    assert!(closed.begin_deactivation(0).unwrap().is_none());
}

#[test]
fn shutdown_preserves_unmatched_results_and_failed_admissions() {
    let mut remote = completed(Owner::new(outcome().binding.unwrap()));
    let mut windows = Windows::default();
    assert!(windows.settle_shutdown(&mut remote, 0).is_err());
    assert!(remote.surface_cleanup_pending());
    assert!(remote.take_surface_outcome().is_some());
    windows.settle_shutdown(&mut remote, 0).unwrap();
    let mut failed = outcome();
    failed.uncertain = true;
    assert!(windows
        .admit(
            failed,
            |_| panic!("unconfirmed mount must not authorize"),
            0
        )
        .is_ok());
    assert!(windows.settle_shutdown(&mut remote, 0).is_err());
    assert!(windows.take_failure().unwrap().outcome.uncertain);
}

#[test]
fn shutdown_closes_mounted_window_without_fabricating_inactive_ack() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut windows = Windows::default();
    assert!(windows.insert(window(&mut grant)).is_ok());
    assert!(windows
        .settle_shutdown(&mut Sessions::default(), 0)
        .is_err());
    let closed = windows.take_closed().unwrap();
    assert!(!closed.is_alive());
    assert_eq!(
        closed.lifecycle().unwrap().binding().lifecycle(),
        Lifecycle::Mounted
    );
}
