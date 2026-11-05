//! Confirm 自动恢复仍只读；暂停撤销追加前参与凭证，但不解绑既有 receiver 或已追加图片。
use super::super::poll::pause;
use super::*;

#[test]
fn receive_policy_confirm_automatically_reattaches_then_saves_and_reports_original_receiver() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = fixture();
        entry(&mut fixture);
        fixture.app.resume_projection_poll_for_test();
        poll(&mut fixture);
        assert!(fixture.app.remote_sessions.projection_pending());
        assert!(fixture.app.authorization.borrow().request.is_none());
        wait_remote(&fixture.app);
        fixture.finish_accept();
        fixture.app.defer_projection_poll_for_test(&fixture.origin);
        until(&mut fixture.app, |app| {
            !app.projection_receive.preparing() && !app.remote_sessions.projection_pending()
        });
        assert!(fixture.unit_present());
        let journal = fixture.journal();
        pause(&mut fixture, true);
        assert_eq!(fixture.journal(), journal);
        receipt(&mut fixture);
        fixture
            .app
            .start_checkpoint_with(&mut |_, _, message, warning| assert!(!warning, "{message}"));
        checkpoint(&mut fixture.app);
        let end = Instant::now() + Duration::from_secs(10);
        while fixture.app.projection_receive.busy() {
            poll(&mut fixture);
            assert!(
                Instant::now() < end,
                "paused existing receiver report deadline"
            );
            std::thread::yield_now();
        }
        fixture.listener = Some(fixture.server.take().unwrap().join().unwrap());
        assert_eq!(
            fixture.app.authorization.borrow().projection.status,
            "投送已呈现"
        );
        assert!(fixture
            .app
            .projection_checkpoint_inclusion(fixture.ticket())
            .is_some());
        assert!(!fixture.store.pending().unwrap()[0].cleanup_pending());
    }
}

#[test]
fn receive_policy_pause_away_back_rejects_late_reattach_without_append_or_unlink() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        for preparing in [false, true] {
            let mut fixture = fixture();
            select(&mut fixture);
            let journal = fixture.journal();
            if preparing {
                poll(&mut fixture);
                let end = Instant::now() + Duration::from_secs(10);
                while !fixture.app.projection_receive.preparation_finished() {
                    assert!(Instant::now() < end, "pause reattach preparation deadline");
                    std::thread::yield_now();
                }
            }
            pause(&mut fixture, true);
            pause(&mut fixture, false);
            poll(&mut fixture);
            assert!(!fixture.unit_present());
            assert_eq!(fixture.app.documents.len(), 1);
            assert!(!fixture.app.projection_receive.busy());
            assert!(!fixture.app.projection_cleanup.pending_local());
            assert_eq!(fixture.journal(), journal);
            assert!(!fixture.store.pending().unwrap()[0].cleanup_pending());
        }
    }
}
