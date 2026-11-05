//! accepted 重建迟到取消必须保 journal/远端关联；与 NEW 的自动补偿责任不同。
use super::*;

#[test]
fn projection_reattach_native_stale_destination_and_reset_preserve_existing_receiver_without_cleanup(
) {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        for mode in 0..3 {
            let mut fixture = fixture();
            let before = fixture.app.documents.len();
            select(&mut fixture);
            let journal = fixture.journal();
            if mode == 0 {
                let record = fixture.app.restored_session.as_mut().unwrap();
                record.edit_original(record.original.to_vec().into());
            } else if mode == 1 {
                fixture.app.reset_projection_inbox();
            } else {
                fixture
                    .app
                    .authorization_owner
                    .invalidate(&fixture.origin)
                    .unwrap();
            }
            until(&mut fixture.app, |app| {
                !app.projection_receive.busy() && !app.remote_sessions.projection_pending()
            });
            fixture.app.projection_cleanup.flush().unwrap();
            assert_eq!(fixture.app.documents.len(), before);
            assert!(!fixture.unit_present());
            assert_eq!(fixture.journal(), journal);
            assert!(!fixture.store.pending().unwrap()[0].cleanup_pending());
            assert!(!fixture.app.projection_cleanup.pending_local());
        }
    }
}

#[test]
fn projection_reattach_native_automatic_reject_mode_uses_read_without_manual_request() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = fixture();
        entry(&mut fixture);
        fixture
            .app
            .authorization
            .borrow_mut()
            .consent
            .install(Policy::Reject);
        fixture.app.resume_projection_poll_for_test();
        poll(&mut fixture);
        assert!(fixture.app.remote_sessions.projection_pending());
        assert!(fixture.app.authorization.borrow().request.is_none());
        wait_remote(&fixture.app);
        fixture.finish_accept();
        until(&mut fixture.app, |app| {
            !app.projection_receive.preparing() && !app.remote_sessions.projection_pending()
        });
        assert!(fixture.unit_present());
        assert!(!fixture.ticket().cleanup_pending());
        assert!(!fixture.app.projection_cleanup.pending_local());
    }
}
