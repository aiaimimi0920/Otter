//! Exit admission and post-publication settlement share the existing hidden owner fixture.
use super::*;

unsafe fn pause(app: &Application) {
    let accepted = crate::desktop::session_end::with_query(
        &app.session_end,
        || true,
        || SendMessageW(app.documents[0].window.0, WM_QUERYENDSESSION, None, None).0,
    );
    assert_eq!(accepted, 1);
    assert!(app.session_end.paused());
}
unsafe fn resume(app: &Application) {
    SendMessageW(
        app.documents[0].window.0,
        WM_ENDSESSION,
        Some(WPARAM(0)),
        None,
    );
    assert!(!app.session_end.paused());
}

#[test]
fn exit_requests_reject_new_language_work_without_queueing_or_writing() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let original = br#"{"schemaVersion":2}"#;
        let fixture = Fixture::new(Some(original));
        let mut app = application(&fixture);
        for stage in 0..3 {
            app.exit_requested.set(stage == 0);
            app.exit_after_save = stage == 1;
            if stage == 2 {
                pause(&app);
            }
            assert_eq!(
                app.request_display_language(request("exit", DisplayLanguage::English, 0)),
                Err("unavailable".into())
            );
            assert!(!app.locale_blocks_exit());
            assert!(app.take_display_language_result().is_none());
            assert_eq!(fs::read(&fixture.0).unwrap(), original);
        }
        resume(&app);
    }
}

#[test]
fn queued_expired_or_cancelled_remote_settles_while_paused_without_writing() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        for expired in [false, true] {
            let original = br#"{"schemaVersion":2}"#;
            let fixture = Fixture::new(Some(original));
            let mut app = application(&fixture);
            app.request_display_language(request("queued", DisplayLanguage::English, 0))
                .unwrap();
            assert!(app.locale_blocks_exit());
            let queued = app.locale.remote.as_mut().unwrap();
            if expired {
                queued.expires_at = Instant::now() - Duration::from_millis(1);
            } else {
                queued.cancel.store(true, Ordering::Release);
            }
            pause(&app);
            app.process_display_language();
            assert!(!app.locale_blocks_exit());
            assert_eq!(
                app.take_display_language_result().unwrap().outcome,
                Outcome::Expired
            );
            assert_eq!(app.display_language_status().revision, 0);
            assert_eq!(fs::read(&fixture.0).unwrap(), original);
            resume(&app);
        }
    }
}

#[test]
fn cancelled_queued_remote_reawakens_exit_without_another_user_message() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let original = br#"{"schemaVersion":2}"#;
        let fixture = Fixture::new(Some(original));
        let mut app = application(&fixture);
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, WM_APP, 0xffff, PM_REMOVE).as_bool() {}
        // Match the main loop after consuming the startup maintenance notification.
        app.execution_wake.acknowledge();
        let request = request("exit-queued", DisplayLanguage::English, 0);
        let cancel = request.cancel.clone();
        app.request_display_language(request).unwrap();
        assert!(PeekMessageW(&mut message, None, WM_APP, 0xffff, PM_REMOVE).as_bool());
        let wake_message = message.message;
        app.execution_wake.acknowledge();
        app.exit_requested.set(true);
        cancel.store(true, Ordering::Release);
        // Match the main loop's order: exit observes the queued work before locale settlement.
        assert!(!app.close_requested_with_notify(
            |_| panic!("queued language work must block exit"),
            |_, error| panic!("{error}"),
        ));
        app.process_display_language();
        assert!(!app.locale_blocks_exit());
        assert!(app.exit_requested.get());
        assert_eq!(
            app.take_display_language_result().unwrap().outcome,
            Outcome::Expired
        );
        // Only the owner's completion wake may resume exit; inject no input or timer messages.
        assert!(PeekMessageW(&mut message, None, wake_message, wake_message, PM_REMOVE).as_bool());
        app.execution_wake.acknowledge();
        assert!(app.close_requested_with_notify(
            |_| super::super::super::exit::Choice::Discard,
            |_, error| panic!("{error}"),
        ));
        assert!(app.exit_committed);
        assert!(!app.documents.iter().any(|document| document.is_alive()));
        assert_eq!(fs::read(&fixture.0).unwrap(), original);
    }
}

#[test]
fn cancellation_after_disk_publication_still_settles_as_applied_while_paused() {
    unsafe {
        let _apartment = crate::desktop::Apartment::new().unwrap();
        let fixture = Fixture::new(Some(br#"{"schemaVersion":2}"#));
        let mut app = application(&fixture);
        let request = request("published", DisplayLanguage::English, 0);
        let cancel = request.cancel.clone();
        app.request_display_language(request).unwrap();
        app.process_display_language();
        assert!(app.locale_busy());
        // Observe the real atomic publication before cancellation; never consume the worker result here.
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let committed = fs::read(&fixture.0)
                .ok()
                .and_then(|bytes| Document::decode(&bytes).ok())
                .is_some_and(|document| document.display_language_revision() == Ok(1));
            if committed {
                break;
            }
            assert!(
                Instant::now() < until,
                "language publication did not finish"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(app.display_language_status().revision, 0);
        cancel.store(true, Ordering::Release);
        pause(&app);
        finish(&mut app);
        let receipt = app.take_display_language_result().unwrap();
        assert_eq!(receipt.outcome, Outcome::Applied);
        assert_eq!(receipt.revision, 1);
        assert_eq!(receipt.language, DisplayLanguage::English);
        assert_eq!(crate::ui_text::current_language(), DisplayLanguage::English);
        let saved = Document::decode(&fs::read(&fixture.0).unwrap()).unwrap();
        assert_eq!(saved.display_language().unwrap(), DisplayLanguage::English);
        assert_eq!(saved.display_language_revision().unwrap(), 1);
        resume(&app);
    }
}
