//! Ready/准备/隐藏追加的终止补偿；保留本地图像，复用原真实 TLS peer 和唯一 cleanup owner。
use super::*;
#[path = "application_projection_receive_persistence_tests.rs"]
mod persistence;

unsafe fn appended(fixture: &mut Fixture) {
    fixture.accepted(true);
    until(&mut fixture.app, |app| !app.projection_receive.preparing());
    assert!(fixture.unit_present());
    assert!(fixture.app.projection_receive.busy());
    assert!(fixture.app.save_job.is_none());
    assert!(fixture
        .app
        .projection_checkpoint_inclusion(fixture.ticket())
        .is_none());
}
fn assert_marker(fixture: &mut Fixture, journal: &[u8], marked: bool) {
    fixture.app.projection_cleanup.flush().unwrap();
    if marked {
        let mut expected: serde_json::Value = serde_json::from_slice(journal).unwrap();
        expected["entries"][0]["cleanup"] = true.into();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fixture.journal()).unwrap(),
            expected
        );
    } else {
        assert_eq!(fixture.journal(), journal);
    }
    assert_eq!(fixture.store.pending().unwrap().len(), 1);
    assert_eq!(
        fixture.store.pending().unwrap()[0].cleanup_pending(),
        marked
    );
    let mut ticket = fixture.store.pending().unwrap().pop().unwrap();
    assert!(ticket.same_intent(fixture.ticket()));
    ticket.activate().unwrap(); // 已交接的 marker 不得仍持原 Hold 活动许可。
}
unsafe fn ready_rejected(case: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    if case == 3 {
        fixture
            .store
            .begin(
                &crate::projection_delivery::accept::tests::invitation(&fixture.origin),
                "device-old",
            )
            .unwrap();
    }
    let ready = fixture.accepted(false).unwrap();
    let journal = fixture.journal();
    if case == 1 {
        fixture.app.projection_receive.waiting(None);
    } else if case == 2 {
        fixture.app.projection_profile.close();
    } else {
        fixture.app.reset_projection_inbox();
    }
    fixture.app.receive_projection_ready(ready);
    assert!(!fixture.app.projection_receive.busy());
    assert!(!fixture.unit_present());
    assert_marker(&mut fixture, &journal, case != 3);
}
#[test]
fn projection_receive_lifecycle_ready_without_waiting_marks_only_original_fresh_ticket() {
    unsafe { ready_rejected(0) }
}
#[test]
fn projection_receive_lifecycle_ready_without_scope_marks_only_original_fresh_ticket() {
    unsafe { ready_rejected(1) }
}
#[test]
fn projection_receive_lifecycle_ready_without_original_profile_still_hands_off_exact_ticket() {
    unsafe { ready_rejected(2) }
}
#[test]
fn projection_receive_lifecycle_reused_ready_never_acquires_fresh_cleanup_qualification() {
    unsafe { ready_rejected(3) }
}

#[test]
fn projection_receive_lifecycle_successful_preparation_stopped_before_poll_releases_then_marks() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        fixture.accepted(true);
        let journal = fixture.journal();
        let end = Instant::now() + Duration::from_secs(10);
        while !fixture.app.projection_receive.preparation_finished() {
            assert!(Instant::now() < end);
            std::thread::yield_now();
        }
        fixture.app.reset_projection_inbox();
        assert!(!fixture.app.projection_cleanup.pending_local());
        let mut ticket = fixture.ticket().retry();
        assert_eq!(
            ticket.activate(),
            Err(crate::projection_delivery::journal::Error::Busy)
        );
        until(&mut fixture.app, |app| !app.projection_receive.busy());
        assert!(!fixture.unit_present());
        assert_marker(&mut fixture, &journal, true);
    }
}
unsafe fn unpublished(case: u8) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    let mut fixture = Fixture::new();
    appended(&mut fixture);
    let journal = fixture.journal();
    match case {
        0 => fixture.app.reset_projection_inbox(),
        1 => {
            fixture
                .app
                .authorization_owner
                .invalidate(&fixture.origin)
                .unwrap();
            fixture.app.process_authorization();
        }
        _ => fixture.app.shutdown(),
    }
    assert!(!fixture.app.projection_receive.busy());
    assert!(fixture.unit_present()); // 自动减权不是删除本地用户数据。
    assert_marker(&mut fixture, &journal, true);
}
#[test]
fn projection_receive_lifecycle_cancel_after_hidden_append_marks_without_removing_local_image() {
    unsafe { unpublished(0) }
}
#[test]
fn projection_receive_lifecycle_revoke_after_hidden_append_marks_without_removing_local_image() {
    unsafe { unpublished(1) }
}
#[test]
fn projection_receive_lifecycle_shutdown_after_hidden_append_persists_fresh_cleanup_marker() {
    unsafe { unpublished(2) }
}

#[test]
fn projection_receive_lifecycle_marker_precedes_original_authenticated_unlink_after_ready_rejection(
) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        let ready = fixture.accepted(false).unwrap();
        fixture.app.reset_projection_inbox();
        fixture.app.receive_projection_ready(ready);
        let mut expected: serde_json::Value = serde_json::from_slice(&fixture.journal()).unwrap();
        expected["entries"][0]["cleanup"] = true.into();
        super::abandonment::unlink_peer(&mut fixture, expected, false);
        let end = Instant::now() + Duration::from_secs(10);
        while !fixture.store.pending().unwrap().is_empty()
            || fixture.app.remote_sessions.projection_pending()
        {
            fixture.app.process_authorization_with_projection(|origin| {
                config(origin, fixture.files.root())
            });
            assert!(
                Instant::now() < end,
                "receive lifecycle authenticated unlink deadline"
            );
            std::thread::yield_now();
        }
        fixture.listener = Some(fixture.server.take().unwrap().join().unwrap());
        assert!(!fixture.unit_present());
        assert!(!fixture.app.projection_cleanup.pending_local());
    }
}
