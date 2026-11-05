//! 本地隐藏窗口验证授权失效的清理；不连接网络或发送生命周期请求。
use super::*;
#[path = "surface_event_delivery_tests.rs"]
mod event_delivery;
#[path = "surface_shutdown_tests.rs"]
mod shutdown;
use crate::desktop::surface_authorized_window::AuthorizedWindow;
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::{Com::*, Threading::WaitForSingleObject},
};
struct Apartment;
impl Apartment {
    fn new() -> Self {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
#[test]
fn revoked_mount_closes_hidden_window_but_keeps_server_lifecycle_evidence() {
    let _com = Apartment::new();
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut outcome = outcome();
    let raw = outcome
        .result
        .as_ref()
        .unwrap()
        .get()
        .replace("\"revision\":1", "\"revision\":57");
    outcome.result = Ok(serde_json::value::RawValue::from_string(raw).unwrap());
    let mut window = AuthorizedWindow::new(&outcome, lease, 0, [200, 140]).unwrap();
    assert_eq!(window.snapshot_revision(), 57);
    assert!(window.is_alive());
    assert_eq!(window.expires_at_ms(), 100);
    assert!(window.show(0).is_err());
    assert!(!window.lifecycle().unwrap().confirmed_active());
    drop(grant);
    assert_eq!(
        unsafe { WaitForSingleObject(HANDLE(window.revocation_handle().as_raw_handle()), 0) },
        WAIT_OBJECT_0
    );
    window.refresh(1).unwrap();
    assert!(!window.is_alive());
    assert_eq!(window.stopped(), Some(Error::Revoked));
    assert_eq!(window.snapshot_revision(), 57);
    assert!(window.show(1).is_err());
    assert!(!window.lifecycle().unwrap().disposed());
    let (owner, lease) = window.into_lifecycle();
    let owner = owner.unwrap();
    assert_eq!(owner.binding(), outcome.binding.as_ref().unwrap());
    assert!(lease.is_revoked());
    assert!(outcome.result.is_ok());
}
#[test]
fn expiry_closes_window_without_claiming_revocation_or_remote_disposal() {
    let _com = Apartment::new();
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut window = AuthorizedWindow::new(&outcome(), lease, 0, [200, 140]).unwrap();
    window.refresh(99).unwrap();
    assert!(window.is_alive());
    window.refresh(100).unwrap();
    assert!(!window.is_alive());
    assert_eq!(window.stopped(), Some(Error::Credential));
    assert!(!window.lifecycle().unwrap().disposed());
    window.refresh(0).unwrap();
    assert!(!window.is_alive());
}

#[test]
fn window_collection_stops_waiting_on_terminal_windows_and_preserves_owners() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut windows = crate::desktop::surface_windows::Windows::default();
    for _ in 0..4 {
        let lease = grant.lease("https://localhost:1234", 0).unwrap();
        let window = AuthorizedWindow::new(&outcome(), lease, 0, [200, 140]).unwrap();
        assert!(windows.insert(window).is_ok());
    }
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let extra = AuthorizedWindow::new(&outcome(), lease, 0, [200, 140]).unwrap();
    let rejected = windows.insert(extra).err().unwrap();
    assert!(rejected.is_alive());
    drop(rejected);
    assert_eq!(windows.wait_handles().len(), 4);
    assert!(windows.deadline(0).unwrap().is_some());
    drop(grant);
    windows.poll(1).unwrap();
    assert!(windows.wait_handles().is_empty());
    assert!(windows.deadline(1).unwrap().is_none());
    for _ in 0..4 {
        assert!(!windows
            .take_closed()
            .unwrap()
            .lifecycle()
            .unwrap()
            .disposed());
    }
    assert!(windows.take_closed().is_none());
}

#[test]
fn admission_retains_unconfirmed_results_and_returns_full_capacity_input() {
    let mut windows = crate::desktop::surface_windows::Windows::default();
    assert!(windows.status().is_none());
    for _ in 0..4 {
        let mut pending = outcome();
        pending.uncertain = true;
        assert!(windows
            .admit(
                pending,
                |_| panic!("unconfirmed result must not request authorization"),
                0
            )
            .is_ok());
    }
    let returned = windows
        .admit(
            outcome(),
            |_| panic!("full collection must not authorize"),
            0,
        )
        .err()
        .unwrap();
    assert!(returned.binding.is_some());
    assert!(windows.wait_handles().is_empty());
    assert!(windows.status().unwrap().contains("未打开"));
    assert!(!windows.can_mount("https://localhost:1234", "node:test"));
    for _ in 0..4 {
        let failure = windows.take_failure().unwrap();
        assert!(failure.outcome.uncertain);
        assert!(!failure.reason.is_empty());
    }
    assert!(windows.take_failure().is_none());
    assert!(windows.can_mount("https://localhost:1234", "node:test"));
}
#[test]
fn confirmed_admission_creates_one_owner_and_keeps_duplicate_evidence() {
    let _com = Apartment::new();
    let mut grant = grant();
    let mut windows = crate::desktop::surface_windows::Windows::default();
    assert!(windows
        .admit(
            outcome(),
            |_| Ok(grant.lease("https://localhost:1234", 0).unwrap()),
            0
        )
        .is_ok());
    assert_eq!(windows.wait_handles().len(), 1);
    assert!(windows
        .admit(
            outcome(),
            |_| panic!("duplicate must not open another window"),
            0
        )
        .is_ok());
    assert_eq!(windows.wait_handles().len(), 1);
    assert!(windows.take_failure().unwrap().outcome.binding.is_some());
    drop(grant);
    windows.poll(1).unwrap();
    let closed = windows.take_closed().unwrap();
    assert_eq!(closed.request().art(), "art:test");
}

#[test]
fn late_activation_ack_preserves_evidence_without_reopening_revoked_window() {
    let _com = Apartment::new();
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut window = AuthorizedWindow::new(&outcome(), lease, 0, [200, 140]).unwrap();
    let mut worker = window.begin_activation(0).unwrap().unwrap();
    assert!(window.lifecycle().is_none());
    assert!(window.begin_activation(0).unwrap().is_none());
    assert!(window.show(0).is_err());
    let state = crate::surface_live_binding::Lifecycle::Active;
    let revision = worker.begin(state).unwrap();
    let binding = worker.binding();
    let bytes = serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(), "attachmentId":binding.attachment(),
        "hookNodeId":binding.node(), "deviceId":binding.device()},
        "lifecycle":"active", "lifecycleRevision":revision}))
    .unwrap();
    worker.acknowledge(&bytes, state, revision).unwrap();
    drop(grant);
    window.refresh(1).unwrap();
    assert!(!window.is_alive());
    assert!(window.finish_activation(worker).is_ok());
    assert!(window.begin_deactivation(1).unwrap().is_none());
    assert!(window.lifecycle().unwrap().confirmed_active());
    assert_eq!(window.binding().lifecycle_revision(), 2);
    assert_eq!(window.snapshot_revision(), 1);
    assert!(window.show(1).is_err());
    assert!(!window.is_alive());
}

#[test]
fn local_close_reports_inactive_once_and_keeps_attachment() {
    let _com = Apartment::new();
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut window = AuthorizedWindow::new(&outcome(), lease, 0, [200, 140]).unwrap();
    assert!(window.begin_deactivation(0).unwrap().is_none());
    window.close().unwrap();
    let mut owner = window.begin_deactivation(0).unwrap().unwrap();
    let state = crate::surface_live_binding::Lifecycle::Inactive;
    let revision = owner.begin(state).unwrap();
    let binding = owner.binding();
    let bytes = serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(), "attachmentId":binding.attachment(),
        "hookNodeId":binding.node(), "deviceId":binding.device()},
        "lifecycle":"inactive", "lifecycleRevision":revision}))
    .unwrap();
    owner.acknowledge(&bytes, state, revision).unwrap();
    let mut windows = crate::desktop::surface_windows::Windows::default();
    assert!(windows.insert(window).is_ok());
    assert!(windows.take_closed().is_none());
    assert!(windows
        .accept_activation(
            crate::surface_lifecycle_runtime::Outcome {
                owner,
                result: Ok(None)
            },
            0,
            false
        )
        .is_ok());
    assert_eq!(windows.status().as_deref(), Some("Surface 窗口已停用"));
    let mut closed = windows.take_closed().unwrap();
    assert!(!closed.lifecycle().unwrap().disposed());
    assert!(!closed.is_alive());
    assert!(closed.begin_deactivation(0).unwrap().is_none());
}

#[test]
fn collection_routes_confirmed_activation_and_returns_unmatched_outcome() {
    let _com = Apartment::new();
    let mut grant = grant();
    let lease = grant.lease("https://localhost:1234", 0).unwrap();
    let mut window = AuthorizedWindow::new(&outcome(), lease, 0, [200, 140]).unwrap();
    let mut owner = window.begin_activation(0).unwrap().unwrap();
    let state = crate::surface_live_binding::Lifecycle::Active;
    let revision = owner.begin(state).unwrap();
    let binding = owner.binding();
    let bytes = serde_json::to_vec(&serde_json::json!({"descriptor":{
        "instanceId":binding.instance(), "attachmentId":binding.attachment(),
        "hookNodeId":binding.node(), "deviceId":binding.device()},
        "lifecycle":"active", "lifecycleRevision":revision}))
    .unwrap();
    owner.acknowledge(&bytes, state, revision).unwrap();
    let mut windows = crate::desktop::surface_windows::Windows::default();
    assert!(windows.insert(window).is_ok());
    assert!(windows
        .accept_activation(
            crate::surface_lifecycle_runtime::Outcome {
                owner,
                result: Ok(None)
            },
            0,
            false
        )
        .is_ok());
    windows.close_all().unwrap();
    let closed = windows.take_closed().unwrap();
    assert!(closed.lifecycle().unwrap().confirmed_active());
    assert!(closed.activation_error().is_none());
    let (owner, _) = closed.into_lifecycle();
    let returned = windows
        .accept_activation(
            crate::surface_lifecycle_runtime::Outcome {
                owner: owner.unwrap(),
                result: Ok(None),
            },
            0,
            false,
        )
        .err()
        .unwrap();
    assert!(returned.owner.confirmed_active());
}
