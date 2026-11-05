//! 恢复资格与 worker 回收的负向边界；失败保图/保 journal，不越过隐藏/撤权/工作区交换。
use super::*;

unsafe fn changed(fixture: &mut Fixture, mode: u8) {
    let doc = fixture
        .app
        .documents
        .iter()
        .find(|doc| doc.state.unit_id.as_str() == fixture.ticket().unit())
        .unwrap();
    match mode {
        0 => doc
            .state
            .canvas
            .borrow_mut()
            .replace_image(crate::image_decode::Image {
                width: 1,
                height: 1,
                pixels: vec![0, 0, 0, 255].into(),
            })
            .unwrap(),
        1 => {
            doc.state.canvas.borrow_mut().selection_drag =
                Some((Default::default(), Default::default()))
        }
        2 => doc.state.group_hidden.set(true),
        3 => doc.state.canvas.borrow_mut().window_opacity = 0.0,
        4 => {
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
        5 => fixture.app.workspace = crate::workspace::Workspace::default(),
        6 => fixture
            .app
            .authorization_owner
            .invalidate(&fixture.origin)
            .unwrap(),
        7 => fixture.app.reset_projection_inbox(),
        _ => unreachable!(),
    }
}
#[test]
fn projection_display_recovery_rejects_changed_display_pending_edit_hidden_zero_opacity_source_and_workspace_before_config(
) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        for mode in 0..6 {
            let mut fixture = Fixture::new();
            saved(&mut fixture);
            let journal = fixture.journal();
            let entry = fixture.app.projection_profile.entries().unwrap()[0].clone();
            changed(&mut fixture, mode);
            fixture
                .app
                .start_projection_display_recovery(entry, &mut |_| {
                    panic!("stale proof reached config")
                });
            assert!(!fixture.app.projection_receive.busy());
            assert_eq!(fixture.journal(), journal);
            assert!(!fixture.app.projection_cleanup.pending_local());
        }
    }
}
#[test]
fn projection_display_recovery_matching_saved_model_with_wrong_pixels_still_fails_original_read_comparison(
) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let journal = fixture.journal();
        changed(&mut fixture, 0);
        fixture
            .app
            .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        checkpoint(&mut fixture.app);
        poll(&mut fixture);
        peer(&mut fixture, 4, false);
        choose(&mut fixture);
        settle(&mut fixture);
        joined(&mut fixture);
        assert_eq!(
            fixture.app.authorization.borrow().projection.status,
            "接收未完成；本地内容和未结意图保留"
        );
        assert_eq!(fixture.journal(), journal);
    }
}
unsafe fn completed(mode: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    saved(&mut fixture);
    let journal = fixture.journal();
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    let origin = fixture.origin.clone();
    fixture.server = Some(std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(head.starts_with("POST /v1/projections/read "));
        reply(
            &mut stream,
            200,
            &serde_json::to_vec(&fixture::response(&origin)).unwrap(),
        );
        listener
    }));
    choose(&mut fixture);
    let handle = fixture.app.projection_receive.wait_handle().unwrap();
    assert_eq!(
        WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
        WAIT_OBJECT_0
    );
    joined(&mut fixture);
    let mut retry = fixture.ticket().retry();
    assert_eq!(
        retry.activate(),
        Err(crate::projection_delivery::journal::Error::Busy)
    );
    let slots = (0..3)
        .map(|_| crate::live_relay_control_runtime::Slot::acquire().unwrap())
        .collect::<Vec<_>>();
    assert!(crate::live_relay_control_runtime::Slot::acquire().is_err());
    drop(slots);
    changed(&mut fixture, mode);
    assert!(fixture.app.projection_receive.busy()); // stop/撤权不提前放原任务槽或活动许可。
    settle(&mut fixture);
    assert_eq!(fixture.journal(), journal);
    assert_ne!(
        fixture.app.authorization.borrow().projection.status,
        "投送已呈现"
    );
    assert!(!fixture.app.projection_cleanup.pending_local());
    retry.activate().unwrap();
}
#[test]
fn projection_display_recovery_late_read_source_change_cannot_show_or_report() {
    unsafe { completed(4) }
}
#[test]
fn projection_display_recovery_late_read_workspace_change_cannot_show_or_report() {
    unsafe { completed(5) }
}
#[test]
fn projection_display_recovery_late_read_revoke_cannot_show_or_report() {
    unsafe { completed(6) }
}
#[test]
fn projection_display_recovery_cancel_retains_completed_slot_and_intent_until_ui_reaps() {
    unsafe { completed(7) }
}
#[test]
fn projection_display_recovery_late_read_changed_display_cannot_show_or_report() {
    unsafe { completed(0) }
}
#[test]
fn projection_display_recovery_cleanup_marker_cannot_be_upgraded_back_to_displayed() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        drop(fixture.ticket().retry().queue_cleanup().unwrap());
        fixture.app.projection_profile.refresh();
        fixture.app.process_authorization();
        let entry = fixture.app.projection_profile.entries().unwrap()[0].clone();
        assert!(entry.cleanup_pending());
        assert!(fixture
            .app
            .projection_checkpoint_inclusion(&entry)
            .is_some());
        fixture
            .app
            .start_projection_display_recovery(entry, &mut |_| panic!("cleanup reached config"));
        assert!(!fixture.app.projection_receive.busy());
        assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
    }
}
