//! 真实 TLS/Ready/Preparing 的迟到围栏；复用原 cleanup，不触碰已接受的保存/恢复状态。
use super::*;
use crate::projection_receive_policy::Policy;
fn policy(app: &mut Application, value: Policy) {
    app.authorization.borrow_mut().request = Some(Request::ReceivePolicy(value));
    app.process_projection_policy();
    let end = Instant::now() + Duration::from_secs(5);
    while app.authorization.borrow().consent.saving {
        assert!(
            Instant::now() < end,
            "receive policy lifecycle save deadline"
        );
        app.process_projection_policy();
        std::thread::yield_now();
    }
    assert_eq!(app.authorization.borrow().consent.policy(), value);
    if value != Policy::Auto {
        let origin = app.authorization.borrow().origin.clone().unwrap();
        app.defer_projection_poll_for_test(&origin);
    }
}
#[test]
fn receive_policy_away_back_discards_late_ready_and_completed_preparation_without_append() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        for preparing in [false, true] {
            let mut fixture = Fixture::new();
            let ready = fixture.accepted(false).unwrap();
            let before = fixture.app.documents.len();
            if preparing {
                fixture.app.receive_projection_ready(ready);
                let end = Instant::now() + Duration::from_secs(10);
                while !fixture.app.projection_receive.preparation_finished() {
                    assert!(
                        Instant::now() < end,
                        "receive policy preparation completion deadline"
                    );
                    std::thread::yield_now();
                }
                policy(&mut fixture.app, Policy::Reject);
                policy(&mut fixture.app, Policy::Confirm);
            } else {
                policy(&mut fixture.app, Policy::Reject);
                policy(&mut fixture.app, Policy::Confirm);
                fixture.app.receive_projection_ready(ready);
            }
            until(&mut fixture.app, |app| !app.projection_receive.busy());
            fixture.app.projection_cleanup.flush().unwrap();
            assert_eq!(fixture.app.documents.len(), before);
            assert!(!fixture.unit_present());
            assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
            assert_eq!(fixture.app.remote_sessions.wait_handles().count(), 0);
        }
    }
}
#[test]
fn receive_policy_reject_after_append_preserves_original_checkpoint_phase_and_binding() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        fixture.accepted(true);
        until(&mut fixture.app, |app| !app.projection_receive.preparing());
        assert!(fixture.unit_present());
        assert!(fixture.app.projection_receive.busy());
        let source = fixture
            .app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .clone();
        let journal = fixture.journal();
        policy(&mut fixture.app, Policy::Reject);
        assert!(fixture.app.projection_receive.busy());
        assert!(fixture.unit_present());
        assert!(fixture
            .app
            .restored_session
            .as_ref()
            .unwrap()
            .original
            .same_storage(&source));
        assert_eq!(fixture.journal(), journal);
        assert!(!fixture.store.pending().unwrap()[0].cleanup_pending());
    }
}

#[test]
fn receive_policy_auto_runs_original_tls_accept_once_without_manual_accept_request() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        fixture
            .app
            .authorization
            .borrow_mut()
            .projection
            .entries
            .push(Arc::new(
                crate::projection_delivery::accept::tests::invitation(&fixture.origin),
            ));
        policy(&mut fixture.app, Policy::Auto);
        fixture
            .app
            .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
        assert!(fixture.app.remote_sessions.projection_pending());
        fixture
            .app
            .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
        assert_eq!(fixture.app.remote_sessions.wait_handles().count(), 1);
        wait_remote(&fixture.app);
        fixture.finish_accept();
        let end = Instant::now() + Duration::from_secs(10);
        loop {
            fixture.app.process_authorization_with_projection(|origin| {
                config(origin, fixture.files.root())
            });
            if !fixture.app.projection_receive.preparing() && fixture.unit_present() {
                break;
            }
            assert!(Instant::now() < end, "automatic receive append deadline");
            std::thread::yield_now();
        }
        assert_eq!(fixture.store.pending().unwrap().len(), 1);
        assert!(!fixture.ticket().cleanup_pending());
        assert!(fixture.app.authorization.borrow().request.is_none());
    }
}

#[test]
fn receive_policy_pause_away_back_abandons_new_ready_before_append() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = Fixture::new();
        let ready = fixture.accepted(false).unwrap();
        super::poll::pause(&mut fixture, true);
        super::poll::pause(&mut fixture, false);
        fixture.app.receive_projection_ready(ready);
        fixture.app.projection_cleanup.flush().unwrap();
        assert_eq!(fixture.app.documents.len(), 1);
        assert!(!fixture.unit_present());
        assert!(fixture.store.pending().unwrap()[0].cleanup_pending());
    }
}
