//! 持续 read 不锁住用户保存/真实工作区替换；迟到结果不能越过旧来源/授权/模态边界。
use super::*;
use crate::desktop::device_authorization_ui::Request;
unsafe fn prepare_save(
    fixture: &mut Fixture,
) -> crate::desktop::application::save_publish::Prepared {
    let path = fixture.files.root().join("sync-manual.json");
    fixture.app.request_save_new(path).unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(prepared) = fixture.app.take_save_candidate().unwrap() {
            return prepared;
        }
        assert!(Instant::now() < end, "sync manual save deadline");
        std::thread::yield_now();
    }
}
unsafe fn swap(fixture: &mut Fixture) {
    let space = fixture
        .app
        .restored_session
        .as_ref()
        .unwrap()
        .space
        .unwrap();
    fixture
        .app
        .request_restore(br#"{"stickers":[],"links":[]}"#.to_vec(), None, space)
        .unwrap();
    let end = Instant::now() + Duration::from_secs(10);
    while fixture.app.restore_job.is_some() {
        fixture
            .app
            .poll_restore_with(false, |_| panic!("explicit coordinates opened a dialog"))
            .unwrap();
        assert!(Instant::now() < end, "sync workspace swap deadline");
        std::thread::yield_now();
    }
    assert!(!fixture.unit_present());
}
unsafe fn late(mode: u8) {
    late_routed(mode, false)
}
unsafe fn late_routed(mode: u8, offline: bool) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = if offline {
        Fixture::offline(false)
    } else {
        Fixture::new()
    };
    saved(&mut fixture);
    let old = source(&fixture);
    let journal = fixture.journal();
    let rx = sync_peer(&mut fixture, vec![1]);
    completed(&mut fixture, &rx);
    joined(&mut fixture);
    assert!(!fixture.app.projection_receive.preparing());
    let modal = std::rc::Rc::new(std::cell::Cell::new(false));
    match mode {
        0 => {
            let prepared = prepare_save(&mut fixture);
            prepared
                .commit_ready(&mut fixture.app, &std::sync::atomic::AtomicBool::new(false))
                .unwrap();
            let bytes = std::fs::read(fixture.files.root().join("sync-manual.json")).unwrap();
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(value["stickers"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["qrProjection"]["revision"] == 5));
        }
        1 => swap(&mut fixture),
        2 => fixture
            .app
            .authorization_owner
            .invalidate(&fixture.origin)
            .unwrap(),
        3 => {
            fixture.app.authorization.borrow_mut().request =
                Some(Request::Select("https://changed.example".into()))
        }
        4 => {
            modal.set(true);
            fixture.app.modals.register(&modal);
        }
        5 => fixture.app.authorization.borrow_mut().request = Some(Request::Cancel),
        _ => unreachable!(),
    }
    poll(&mut fixture);
    assert!(!fixture.app.projection_receive.busy());
    if mode != 1 {
        assert_eq!(binding(&fixture)["revision"], 5);
        if mode != 0 {
            assert!(source(&fixture).same_storage(&old));
        }
    }
    assert_eq!(fixture.journal(), journal);
    assert!(!fixture.app.projection_cleanup.pending_local());
    let mut ticket = fixture.ticket().retry();
    ticket.activate().unwrap();
}
#[test]
fn projection_sync_late_read_does_not_block_save_or_replace_committed_source() {
    unsafe { late(0) }
}
#[test]
fn projection_sync_late_read_does_not_block_real_workspace_swap() {
    unsafe { late(1) }
}
#[test]
fn projection_sync_late_read_revocation_cannot_install_frame() {
    unsafe { late(2) }
}
#[test]
fn projection_sync_late_read_origin_selection_cannot_install_frame() {
    unsafe { late(3) }
}
#[test]
fn projection_sync_late_read_modal_cannot_install_frame() {
    unsafe { late(4) }
}
#[test]
fn projection_sync_late_read_cancel_releases_original_permit_only_on_join() {
    unsafe { late(5) }
}
#[test]
fn projection_offline_sync_late_read_cannot_cross_save_workspace_grant_origin_or_cancel() {
    unsafe {
        for mode in 0..6 {
            late_routed(mode, true);
        }
    }
}
#[test]
fn projection_offline_sync_wrong_peer_keeps_original_image_binding_and_journal() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::offline(false);
        saved(&mut fixture);
        let old = source(&fixture);
        let journal = fixture.journal();
        let rx = sync_peer(&mut fixture, vec![6]);
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        assert!(source(&fixture).same_storage(&old));
        assert_eq!(binding(&fixture)["revision"], 5);
        assert_ne!(binding(&fixture)["stopped"], true);
        assert_eq!(fixture.journal(), journal);
        assert!(!fixture.app.projection_cleanup.pending_local());
    }
}
#[test]
fn projection_sync_rejected_display_does_not_poison_ui_busy_or_stop_continuous_reads() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        saved(&mut fixture);
        let rx = sync_peer(&mut fixture, vec![1, 0]);
        completed(&mut fixture, &rx);
        super::super::fixture::choose(&mut fixture);
        assert!(!fixture.app.projection_receive.busy());
        assert!(!fixture.app.authorization.borrow().projection.busy);
        assert_eq!(binding(&fixture)["revision"], 6);
        cycle(&mut fixture, &rx);
        joined(&mut fixture);
        assert!(!fixture.app.authorization.borrow().projection.busy);
        assert_eq!(binding(&fixture)["revision"], 6);
    }
}
