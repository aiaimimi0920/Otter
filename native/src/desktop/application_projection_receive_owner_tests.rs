//! 真实授权/TLS→唯一生产 owner→隐藏追加/checkpoint，及取消/来源/撤权/失败/退出保护。
use super::*;
use crate::desktop::application::projection;
#[path = "application_projection_abandonment_tests.rs"]
mod abandonment;
#[path = "application_projection_display_recovery_tests.rs"]
mod display_recovery;
#[path = "application_projection_receive_owner_fixture.rs"]
mod fixture;
#[path = "application_projection_receive_lifecycle_tests.rs"]
mod lifecycle;
#[path = "application_projection_policy_lifecycle_tests.rs"]
mod policy_lifecycle;
#[path = "application_projection_poll_tests.rs"]
mod poll;
#[path = "application_projection_reattach_tests.rs"]
mod reattach;
#[path = "application_projection_visible_tests.rs"]
mod visible;
use fixture::{checkpoint, until, Fixture};

unsafe fn run(case: u8) {
    run_routed(case, false)
}
unsafe fn run_routed(case: u8, offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = if offline {
        Fixture::offline(false)
    } else {
        Fixture::new()
    };
    if case == 9 {
        fixture
            .app
            .restored_session
            .as_mut()
            .unwrap()
            .edit_original(b"invalid append source".to_vec().into());
    }
    let ready = fixture.accepted(case == 0);
    let journal = fixture.journal();
    if case == 1 {
        let source = fixture
            .app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .to_vec();
        fixture
            .app
            .restored_session
            .as_mut()
            .unwrap()
            .edit_original(source.into());
    }
    if let Some(ready) = ready {
        fixture.app.receive_projection_ready(ready);
    }
    if case == 1 {
        assert!(!fixture.app.projection_receive.busy());
        assert!(!fixture.unit_present());
    } else if case == 2 {
        let source = fixture
            .app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .to_vec();
        fixture
            .app
            .restored_session
            .as_mut()
            .unwrap()
            .edit_original(source.into());
        until(&mut fixture.app, |app| !app.projection_receive.busy());
        assert!(!fixture.unit_present());
    } else if case == 3 || case == 9 {
        assert!(fixture.app.projection_receive.preparing());
        assert!(fixture.app.capture_session_snapshot().is_err());
        assert!(!restore_batch::guard::is_idle(&fixture.app));
        if case == 3 {
            fixture.app.reset_projection_inbox();
        }
        // 有意不 poll UI：后台错误/取消已经结束后，原槽与许可仍须由未领取的 Job 保留。
        let end = Instant::now() + Duration::from_secs(10);
        while !fixture.app.projection_receive.preparation_finished() {
            assert!(Instant::now() < end, "unreaped preparation deadline");
            std::thread::yield_now();
        }
        {
            let slots = (0..3)
                .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
                .collect::<Vec<_>>();
            assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
            drop(slots);
        }
        assert!(fixture.app.projection_receive.preparing());
        if case == 3 {
            assert!(fixture
                .app
                .projection_receive
                .deadline(crate::desktop::surface_windows::now_ms().unwrap())
                .is_none());
        }
        let mut retry = fixture.ticket().retry();
        assert_eq!(
            retry.activate(),
            Err(crate::projection_delivery::journal::Error::Busy)
        );
        until(&mut fixture.app, |app| !app.projection_receive.busy());
        fixture.app.projection_cleanup.flush().unwrap();
        assert_eq!(
            retry.activate(),
            Err(crate::projection_delivery::journal::Error::Changed)
        );
        fixture
            .store
            .pending()
            .unwrap()
            .pop()
            .unwrap()
            .activate()
            .unwrap();
        assert!(!fixture.unit_present());
    } else if case == 4 {
        fixture
            .app
            .authorization_owner
            .invalidate(&fixture.origin)
            .unwrap();
        until(&mut fixture.app, |app| !app.projection_receive.busy());
        assert!(!fixture.unit_present());
    } else if case == 5 {
        fixture.app.shutdown();
        assert!(!fixture.app.projection_receive.busy());
        assert!(!fixture.unit_present());
    } else if case == 7 {
        for doc in &fixture.app.documents {
            DestroyWindow(doc.window.0).unwrap();
        }
        // 与 run_loop 相同，先消费真实 WM_QUIT，再进入空工作区收尾；不跳过主循环前提。
        let mut message = MSG::default();
        let end = Instant::now() + Duration::from_secs(10);
        loop {
            let status = fixture.app.next_live_message(&mut message).unwrap();
            assert!(status >= 0);
            if status == 0 {
                assert_eq!(message.wParam.0, 0);
                break;
            }
            assert!(Instant::now() < end, "last Unit quit deadline");
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        fixture.app.finish_deleted_workspace().unwrap();
        assert!(!fixture.app.projection_receive.busy());
        assert!(fixture.app.workspace.units().is_empty());
        let bytes = std::fs::read(fixture.files.root().join("workspace.json")).unwrap();
        assert!(crate::session_document::decode(&bytes)
            .unwrap()
            .workspace
            .units()
            .is_empty());
    } else {
        until(&mut fixture.app, |app| !app.projection_receive.preparing());
        assert!(fixture.unit_present());
        assert!(fixture.app.projection_receive.busy());
        assert!(fixture
            .app
            .projection_receive
            .deadline(crate::desktop::surface_windows::now_ms().unwrap())
            .is_some());
        assert!(fixture
            .app
            .projection_checkpoint_inclusion(fixture.ticket())
            .is_none());
        assert!(fixture
            .app
            .documents
            .iter()
            .all(|doc| !IsWindowVisible(doc.window.0).as_bool()));
        if case == 6 {
            let path = fixture.files.root().join("workspace.json");
            std::fs::write(&path, b"competing checkpoint").unwrap();
            let end = Instant::now() + Duration::from_secs(10);
            let mut warnings = 0;
            loop {
                fixture.app.pending_workspace_save_with(|_, _, _, warning| {
                    warnings += usize::from(warning)
                });
                if fixture.app.save_job.is_none() {
                    break;
                }
                assert!(Instant::now() < end);
                std::thread::yield_now();
            }
            fixture.app.process_authorization();
            assert_eq!(warnings, 1);
            assert!(!fixture.app.projection_receive.busy());
            assert!(fixture
                .app
                .projection_checkpoint_inclusion(fixture.ticket())
                .is_none());
            assert_eq!(std::fs::read(path).unwrap(), b"competing checkpoint");
        } else {
            let mut retry = fixture.ticket().retry();
            assert_eq!(
                retry.activate(),
                Err(crate::projection_delivery::journal::Error::Busy)
            );
            {
                let slots = (0..3)
                    .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
                    .collect::<Vec<_>>();
                assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
                drop(slots);
            }
            checkpoint(&mut fixture.app);
            let binding = fixture
                .app
                .projection_checkpoint_inclusion(fixture.ticket())
                .unwrap();
            let value = serde_json::to_value(binding).unwrap();
            assert_eq!(
                value["otterNativeReceive"]["intentId"],
                fixture.ticket().intent_id()
            );
            assert_eq!(
                fixture.app.authorization.borrow().projection.status,
                "正在保存接收投送…"
            );
            assert!(fixture.app.projection_receive.busy());
            assert_eq!(
                retry.activate(),
                Err(crate::projection_delivery::journal::Error::Busy)
            );
            if case == 8 {
                fixture.app.reset_projection_inbox();
            } else {
                fixture
                    .app
                    .authorization_owner
                    .invalidate(&fixture.origin)
                    .unwrap();
                fixture.app.process_authorization();
            }
            assert!(!fixture.app.projection_receive.busy());
            retry.activate().unwrap();
            assert!(fixture.unit_present());
        }
    }
    fixture.app.projection_cleanup.flush().unwrap();
    if case == 0 || case == 8 {
        assert_eq!(fixture.journal(), journal);
    } else {
        let mut expected: serde_json::Value = serde_json::from_slice(&journal).unwrap();
        expected["entries"][0]["cleanup"] = true.into();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fixture.journal()).unwrap(),
            expected
        );
        assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
    }
    assert_eq!(fixture.store.pending().unwrap().len(), 1);
}

#[test]
fn projection_receive_owner_tls_to_hidden_append_and_durable_checkpoint_holds_grant_capacity_and_intent(
) {
    unsafe { run(0) }
}
#[test]
fn projection_receive_owner_cannot_retarget_ready_to_equal_bytes_in_a_new_source() {
    unsafe { run(1) }
}
#[test]
fn projection_receive_owner_preparation_rechecks_original_source_before_appending() {
    unsafe { run(2) }
}
#[test]
fn projection_receive_owner_cancel_keeps_capacity_and_blocks_capture_until_worker_is_reaped() {
    unsafe { run(3) }
}
#[test]
fn projection_receive_owner_old_grant_revocation_cannot_publish_prepared_pixels() {
    unsafe { run(4) }
}
#[test]
fn projection_receive_owner_shutdown_reaps_preparation_and_marks_without_retiring_journal() {
    unsafe { run(5) }
}
#[test]
fn projection_receive_owner_checkpoint_failure_keeps_local_unit_and_intent_without_saved_success() {
    unsafe { run(6) }
}
#[test]
fn projection_receive_owner_last_unit_drain_cannot_append_old_preparation_into_empty_workspace() {
    unsafe { run(7) }
}
#[test]
fn projection_receive_owner_cancel_after_durable_checkpoint_keeps_local_image_and_intent() {
    unsafe { run(8) }
}
#[test]
fn projection_receive_owner_preparation_failure_retains_capacity_and_intent_until_reaped() {
    unsafe { run(9) }
}
#[test]
fn projection_offline_owner_workspace_replacement_and_cancel_reject_late_preparation() {
    unsafe {
        for case in [1, 2, 3, 8, 9] {
            run_routed(case, true);
        }
    }
}
#[test]
fn projection_offline_owner_revocation_shutdown_and_last_unit_drain_cannot_append() {
    unsafe {
        for case in [4, 5, 7] {
            run_routed(case, true);
        }
    }
}
#[test]
fn projection_offline_owner_checkpoint_failure_keeps_local_image_without_saved_success() {
    unsafe { run_routed(6, true) }
}
