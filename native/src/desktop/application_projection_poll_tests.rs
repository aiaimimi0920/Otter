//! Automatic queries and pause generations in native manual mode; real TLS single slot, independent of appendable targets or global input.
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
        fixture.app.restored_session = None; // Querying/displaying invitations awaiting confirmation must work even without a save target.
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

fn reply_next(fixture: &mut Fixture, path: &'static str, status: u16, body: &'static [u8]) {
    let listener = fixture.listener.take().unwrap();
    let tls = fixture.tls.clone();
    fixture.server = Some(std::thread::spawn(move || {
        let mut stream = StreamOwned::new(ServerConnection::new(tls).unwrap(), accept(&listener));
        let (head, _) = request(&mut stream);
        assert!(
            head.starts_with(&format!("POST {path} ")),
            "expected {path}"
        );
        reply(&mut stream, status, body);
        listener
    }));
}

#[test]
fn missing_cleanup_records_do_not_postpone_or_preempt_inbox_presence() {
    let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _com = Apartment::new().unwrap();
    const INBOX: &str = "/v1/projections/inbox";
    const UNLINK: &str = "/v1/projections/unlink";
    const EMPTY: &[u8] = br#"{"invitations":[]}"#;
    const NOT_FOUND: &[u8] = br#"{"error":{"code":"projection_not_found"}}"#;
    unsafe {
        let mut fixture = Fixture::with_request(INBOX, |stream, _| reply(stream, 200, EMPTY));
        enable(&mut fixture.app);
        poll(&mut fixture);
        wait_remote(&fixture.app);
        joined(&mut fixture);
        poll(&mut fixture);
        fixture.app.defer_projection_poll_for_test(&fixture.origin);
        let mut records = Vec::new();
        for id in 1..=2 {
            let mut wire = crate::projection_delivery::tests::invitation(&fixture.origin, false);
            wire["envelope"]["projectionId"] = format!("projection:{id:032x}").into();
            records.push(wire);
        }
        let inbox = crate::projection_delivery::Inbox::parse(
            &serde_json::to_vec(&serde_json::json!({"invitations":records})).unwrap(),
            &fixture.origin,
        )
        .unwrap();
        for item in inbox.entries() {
            drop(
                fixture
                    .store
                    .begin(item, "device-old")
                    .unwrap()
                    .queue_cleanup()
                    .unwrap(),
            );
        }
        fixture.app.projection_profile.refresh();
        let journal = fixture.journal();

        reply_next(&mut fixture, UNLINK, 404, NOT_FOUND);
        poll(&mut fixture);
        wait_remote(&fixture.app);
        joined(&mut fixture);
        // Make inbox due just as the first cleanup completes; the second is also due.
        enable(&mut fixture.app);
        fixture.app.poll_projection_inbox();
        assert!(
            fixture.app.projection_policy_deadline().is_none(),
            "404 from old unlink must not impose inbox backoff"
        );

        reply_next(&mut fixture, INBOX, 200, EMPTY);
        poll(&mut fixture);
        wait_remote(&fixture.app);
        joined(&mut fixture);
        fixture.app.poll_projection_inbox();
        let inbox_due = fixture.app.projection_policy_deadline().unwrap();
        assert!(inbox_due > Instant::now());

        reply_next(&mut fixture, UNLINK, 404, NOT_FOUND);
        poll(&mut fixture);
        wait_remote(&fixture.app);
        joined(&mut fixture);
        fixture.app.poll_projection_inbox();
        assert_eq!(fixture.app.projection_policy_deadline(), Some(inbox_due));
        assert_eq!(
            fixture.journal(),
            journal,
            "404 is not authoritative unlink success"
        );
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
        joined(&mut fixture); // A completed old snapshot still occupies its original slot; neither pause nor resume may revive it.
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
