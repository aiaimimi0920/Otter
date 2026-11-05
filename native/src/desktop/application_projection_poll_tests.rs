//! 原生手动模式自动查询及暂停代次；真实 TLS 单槽，不依赖可追加目标或全局输入。
use super::*;

fn enable(app: &mut Application) {
    app.resume_projection_poll_for_test();
}
fn poll(fixture: &mut Fixture) {
    fixture
        .app
        .process_authorization_with_projection(|origin| config(origin, fixture.files.root()));
}
pub(super) fn pause(fixture: &mut Fixture, paused: bool) {
    fixture.app.authorization.borrow_mut().request = Some(Request::ReceivePause(paused));
    fixture.app.process_projection_policy();
    let end = Instant::now() + Duration::from_secs(5);
    while fixture.app.authorization.borrow().consent.saving {
        fixture.app.process_projection_policy();
        assert!(Instant::now() < end, "receive pause commit deadline");
        std::thread::yield_now();
    }
    assert_eq!(fixture.app.authorization.borrow().consent.paused(), paused);
    fixture.app.defer_projection_poll_for_test(&fixture.origin);
}
unsafe fn inbox() -> Fixture {
    Fixture::with_request("/v1/projections/inbox", |stream, origin| {
        reply(
            stream,
            200,
            &crate::projection_delivery::tests::body(origin),
        );
    })
}
fn joined(fixture: &mut Fixture) {
    fixture.listener = Some(fixture.server.take().unwrap().join().unwrap());
}

#[test]
fn receive_policy_confirm_automatically_polls_without_append_scope_and_never_accepts_new() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = inbox();
        fixture.app.restored_session = None; // 没有保存目标也必须能查询/显示待确认邀请。
        enable(&mut fixture.app);
        poll(&mut fixture);
        assert!(fixture.app.authorization.borrow().request.is_none());
        assert_eq!(fixture.app.remote_sessions.wait_handles().count(), 1);
        wait_remote(&fixture.app);
        joined(&mut fixture);
        poll(&mut fixture);
        assert_eq!(
            fixture.app.authorization.borrow().projection.entries.len(),
            1
        );
        assert!(!fixture.app.remote_sessions.projection_pending());
        assert!(fixture.store.pending().unwrap().is_empty());
        let listener = fixture.listener.take().unwrap();
        let tls = fixture.tls.clone();
        fixture.server = Some(std::thread::spawn(move || {
            let mut stream =
                StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
            let (head, body) = request(&mut stream);
            assert!(head.starts_with("POST /v1/projections/inbox "));
            assert_eq!(body, serde_json::json!({"policy":"confirm"}));
            reply(&mut stream, 200, br#"{"invitations":[]}"#);
            listener
        }));
        fixture.app.process_projection_automatic(
            &mut |origin| config(origin, fixture.files.root()),
            Instant::now() + Duration::from_secs(4),
        );
        wait_remote(&fixture.app);
        joined(&mut fixture);
        poll(&mut fixture);
        assert!(fixture
            .app
            .authorization
            .borrow()
            .projection
            .entries
            .is_empty());
        assert_eq!(fixture.app.documents.len(), 1);
        assert!(fixture.store.pending().unwrap().is_empty());
    }
}

#[test]
fn receive_policy_pause_suppresses_polling_and_away_back_discards_completed_inbox() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    unsafe {
        let mut fixture = inbox();
        enable(&mut fixture.app);
        poll(&mut fixture);
        wait_remote(&fixture.app);
        joined(&mut fixture); // 已经完成的旧快照仍占原槽，暂停/恢复都不能使它复活。
        pause(&mut fixture, true);
        for _ in 0..1000 {
            fixture.app.process_projection_automatic(
                &mut |_| panic!("paused poll read config"),
                Instant::now() + Duration::from_secs(7200),
            );
        }
        pause(&mut fixture, false);
        fixture
            .app
            .process_authorization_with_projection(|_| panic!("stale inbox poll prepared client"));
        assert!(fixture
            .app
            .authorization
            .borrow()
            .projection
            .entries
            .is_empty());
        assert!(!fixture.app.remote_sessions.projection_pending());
        assert_eq!(fixture.app.documents.len(), 1);
        assert!(fixture.store.pending().unwrap().is_empty());
    }
}
