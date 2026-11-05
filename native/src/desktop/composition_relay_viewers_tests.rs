use super::*;
use crate::device_authorization_lease::Grant;
use std::os::windows::io::AsRawHandle;
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::Threading::WaitForSingleObject,
};
fn config() -> Config {
    Config::new(
        "https://localhost:9",
        std::path::Path::new("C:/Users/Public/nas_home/AI/GameEditor/linshi/unused-live-viewer"),
        "test",
    )
    .unwrap()
}
fn binding() -> Binding {
    Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:1","generation":1},"attachments":{"attach:1":{"descriptor":{"attachmentId":"attach:1","instanceId":"instance:1","hookNodeId":"node:1","deviceId":"device-1"},"lifecycle":"active","lifecycleRevision":2}}}}"#, "https://localhost:9","device-1","node:1").unwrap()
}
fn target() -> Target {
    let snapshot=crate::live_relay_viewer_identity::Snapshot::parse(br#"{"session":{"protocolVersion":"loom.live.v1","sessionId":"live:1","sourceDeviceId":"source:1","sourceHookId":"hook:1","revision":1,"viewerDevices":[]},"epoch":1,"closed":false}"#,"live:1").unwrap();
    binding()
        .viewer_target(
            snapshot.identity(),
            "https://localhost:9",
            "device-1",
            "node:1",
            1,
        )
        .unwrap()
}
fn lease() -> Lease {
    let mut grant = Grant::new(crate::device_session_client::tests::session(
        "https://localhost:9",
        u64::MAX,
    ));
    let lease = grant.lease("https://localhost:9", 0).unwrap();
    lease.revoke();
    lease
}
fn reap(sessions: &mut Sessions) {
    let handles: Vec<_> = sessions
        .wait_handles()
        .map(|handle| HANDLE(handle.as_raw_handle()))
        .collect();
    for handle in handles {
        assert_eq!(unsafe { WaitForSingleObject(handle, 5000) }, WAIT_OBJECT_0);
    }
    assert_eq!(sessions.poll().len(), 1);
}
#[test]
fn duplicate_and_stale_stop_cannot_replace_or_stop_new_owner() {
    let mut sessions = Sessions::default();
    sessions
        .start_viewing(config(), lease(), target(), binding())
        .unwrap();
    let old = sessions.viewers().pop().unwrap().key;
    assert!(
        matches!(sessions.start_viewing(config(),lease(),target(),binding()),Err(error) if error.kind()==io::ErrorKind::AlreadyExists)
    );
    assert!(sessions.stop_viewer(&old));
    assert!(sessions.viewers()[0].stopping);
    reap(&mut sessions);
    sessions
        .start_viewing(config(), lease(), target(), binding())
        .unwrap();
    assert!(!sessions.stop_viewer(&old));
    assert!(!sessions.viewers()[0].stopping);
    sessions.cancel_pending_viewers(|_| false);
    assert!(sessions.viewers()[0].stopping);
    reap(&mut sessions);
    assert!(sessions.finished());
    assert_eq!(sessions.wait_handles().count(), 0);
}
#[test]
fn mismatched_surface_and_unselected_target_never_start_worker() {
    let mut sessions = Sessions::default();
    for mode in 0..3 {
        let mut target = target();
        if mode == 0 {
            target.surface = "different".into();
        }
        if mode == 1 {
            target.attachment = "different".into();
        }
        if mode == 2 {
            target.selected = None;
        }
        assert!(
            matches!(sessions.start_viewing(config(),lease(),target,binding()),Err(error) if error.kind()==io::ErrorKind::InvalidInput)
        );
    }
    assert!(sessions.finished());
    assert!(sessions.viewers().is_empty());
}
#[test]
fn readonly_policy_explicitly_handles_known_roles_and_rejects_unknown_business() {
    let event = |kind| {
        serde_json::from_value(serde_json::json!({"protocolVersion":"loom.live.v1","sessionId":"live:1","epoch":1,"sequence":1,"messageType":kind,"payload":{}})).unwrap()
    };
    for kind in [
        "session_state",
        "observation",
        "trigger_event",
        "input_event",
        "trigger_condition",
    ] {
        assert_eq!(read_only(&[event(kind)]), Ok(()));
    }
    assert_eq!(read_only(&[event("surface_action")]), Err(()));
}
