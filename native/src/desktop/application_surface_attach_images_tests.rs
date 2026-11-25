//! Actual Application attach/activation/resource/close chain; no fabricated Outcome or ACK.
use super::*;
#[path = "application_surface_attach_images_peer.rs"]
mod peer;

fn run(revoke_ready: bool, reject_window: bool) {
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, server, pem) = fixture_with_pem();
    let origin = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let peer =
        std::thread::spawn(move || peer::serve(listener, server, revoke_ready, reject_window));
    unsafe {
        let mut app = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew { client, store })
            .unwrap();
        {
            let handle = app.authorization_owner.wait_handles().next().unwrap();
            assert_eq!(
                windows::Win32::System::Threading::WaitForSingleObject(
                    HANDLE(handle.as_raw_handle()),
                    5000
                ),
                WAIT_OBJECT_0
            );
        }
        assert!(matches!(
            app.authorization_owner.poll().as_slice(),
            [crate::device_authorization_owner::Event::Authorized { .. }]
        ));
        let mut reads = 0;
        let mut config = |requested: &str| {
            assert_eq!(requested, origin);
            reads += 1;
            fixture::config(requested, files.root())
        };
        super::super::super::super::surface_start::start_with(
            &origin,
            "art:test".into(),
            "node:test".into(),
            &mut app.authorization_owner,
            &mut app.remote_sessions,
            &app.surface_windows,
            &mut config,
        )
        .unwrap();
        fixture::wait(&app);
        if reject_window {
            // The result is confirmed, but the authorization view closed before window admission.
            app.authorization.borrow_mut().closed = true;
        }
        app.poll_remote_live_with(false, &mut config).unwrap();
        if reject_window {
            assert!(app.surface_windows.iter_mut().next().is_none());
            assert!(
                app.surface_windows.take_failure().is_none(),
                "owner still in flight"
            );
            assert_eq!(app.remote_sessions.wait_handles().count(), 1);
            fixture::wait(&app);
            app.poll_remote_live_with(false, |_| panic!("inactive retried"))
                .unwrap();
            assert!(app.surface_windows.take_failure().is_none());
            assert!(app.surface_windows.can_mount(&origin, "node:test"));
            let records = app.surface_windows.retired_records();
            assert_eq!(records.len(), 1);
            let saved: serde_json::Value =
                serde_json::from_slice(&crate::surface_recovery_record::encode(records).unwrap())
                    .unwrap();
            assert_eq!(saved["records"][0]["binding"]["revision"], 2);
            assert_eq!(saved["records"][0]["binding"]["lifecycle"], "inactive");
            assert_eq!(app.authorization.borrow().recent_surface_recovery.len(), 1);
            assert_eq!(app.remote_sessions.wait_handles().count(), 0);
            let listener = peer.join().unwrap();
            assert!(
                matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock)
            );
            return;
        }
        // Admission creates the real hidden HWND; only activation is in flight, no GET yet.
        let parent = {
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert!(window.is_alive() && !window.is_shown());
            assert!(window.lifecycle().is_none());
            assert_eq!(window.installed_images_for_test(), 0);
            window.edit_test_parent()
        };
        assert_eq!(app.remote_sessions.wait_handles().count(), 1);
        fixture::wait(&app);
        app.poll_remote_live_with(false, &mut config).unwrap();
        {
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert!(window.lifecycle().unwrap().confirmed_active());
            assert_eq!(window.image_placeholder_for_test("inline"), Some(true));
            assert_eq!(window.installed_images_for_test(), 0);
        }
        // Exactly attach + activation + resource configuration; inline must not read it.
        assert_eq!(reads, 3);
        fixture::wait(&app);
        if revoke_ready {
            app.authorization_owner.close();
            app.poll_remote_live_with(false, |_| panic!("revoked image read configuration"))
                .unwrap();
        } else {
            app.poll_remote_live_with(false, |_| panic!("inline read network configuration"))
                .unwrap();
            assert_eq!(
                app.surface_windows
                    .iter_mut()
                    .next()
                    .unwrap()
                    .installed_images_for_test(),
                1
            );
            fixture::wait(&app);
            app.poll_remote_live_with(false, |_| panic!("image was requested twice"))
                .unwrap();
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert_eq!(window.installed_images_for_test(), 2);
            assert_eq!(
                window.installed_image_size_for_test("inline"),
                Some([2., 1.])
            );
            assert_eq!(window.image_placeholder_for_test("inline"), Some(false));
            // Nonempty resourceId takes priority without src fallback; nodes lacking leases never enter the visible projection.
            assert_eq!(window.image_placeholder_for_test("missing"), None);
            SendMessageW(parent, WM_PAINT, None, None);
            assert!(window.take_error().is_none());
            assert_eq!(window.uploaded_images_for_test(), 2);
            for _ in 0..3 {
                app.poll_remote_live_with(false, |_| panic!("idle image read configuration"))
                    .unwrap();
            }
            app.surface_windows.close_all().unwrap();
            app.poll_remote_live_with(false, |o| fixture::config(o, files.root()))
                .unwrap();
            assert!(!app.surface_windows.can_mount(&origin, "node:test"));
            assert!(app.surface_windows.retired_records().is_empty());
            fixture::wait(&app);
            app.poll_remote_live_with(false, |_| panic!("inactive image read configuration"))
                .unwrap();
        }
        assert!(!IsWindow(Some(parent)).as_bool());
        if revoke_ready {
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert!(!window.is_alive());
            assert_eq!(window.installed_images_for_test(), 0);
            assert!(window.lifecycle().unwrap().confirmed_active());
            assert!(!app.surface_windows.can_mount(&origin, "node:test"));
            assert!(app.surface_windows.retired_records().is_empty());
        } else {
            assert!(app.surface_windows.iter_mut().next().is_none());
            assert!(app.surface_windows.can_mount(&origin, "node:test"));
            let records = app.surface_windows.retired_records();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].node(), "node:test");
            assert_eq!(records[0].art(), Some("art:test"));
            assert!(!records[0].uncertain() && !records[0].in_flight());
            reopen_from_current_session(&mut app, &origin, files.root());
        }
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        let listener = peer.join().unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        app.authorization_owner.close();
        app.remote_sessions.close();
        assert!(app.remote_sessions.poll_surfaces().is_empty());
        assert!(app.remote_sessions.finished());
    }
}
/// Actual menu routing and application requests over TLS; inject only the dialog answer and test CA.
unsafe fn reopen_from_current_session(app: &mut Application, origin: &str, root: &std::path::Path) {
    use crate::desktop::device_authorization_ui as ui;
    let doc = &app.documents[0];
    let menu = CreatePopupMenu().unwrap();
    let command = ui::TEST_RECENT_RECOVERY;
    ui::append_menu(menu, &doc.state).unwrap();
    assert_eq!(
        GetMenuState(menu, command as u32, MF_BYCOMMAND) & MF_GRAYED.0,
        0
    );
    DestroyMenu(menu).unwrap();
    assert!(crate::desktop::application::handles(command));
    assert!(crate::desktop::group_lock::allows(command));
    ui::execute(doc.window.0, &doc.state, command).unwrap();
    assert!(
        matches!(app.authorization.borrow().request.as_ref(), Some(ui::Request::QueryRecovery(record)) if record.node() == "node:test")
    );
    let mut config = |selected: &str| {
        assert_eq!(selected, origin);
        fixture::config(selected, root)
    };
    app.process_authorization_with_projection(&mut config);
    assert!(app.authorization.borrow().recovery_busy);
    fixture::wait(app);
    app.poll_remote_live_with(false, |_| panic!("query must not start remount"))
        .unwrap();
    assert!(!app.remote_sessions.attaching());
    assert!(app.surface_windows.iter_mut().next().is_none());
    let doc = &app.documents[0];
    ui::confirm_recovery_for_test(doc.window.0, &doc.state, |_| false).unwrap();
    assert!(app.authorization.borrow().request.is_none());
    ui::confirm_recovery_for_test(doc.window.0, &doc.state, |candidate| {
        assert_eq!(candidate.instance(), "instance:test");
        assert_eq!(candidate.attachment(), "attachment:test");
        true
    })
    .unwrap();
    app.process_authorization_with_projection(&mut config);
    assert!(app.authorization.borrow().recovery_candidate.is_none());
    assert!(app.remote_sessions.attaching());
    for _ in 0..2 {
        fixture::wait(app);
        app.poll_remote_live_with(false, &mut config).unwrap();
    }
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(window.is_alive() && window.lifecycle().unwrap().confirmed_active());
    assert_eq!(window.binding().instance(), "instance:test");
    assert_eq!(window.binding().attachment(), "attachment:test");
    assert_eq!(window.binding().lifecycle_revision(), 5);
    assert_eq!(window.snapshot_revision(), 2);
    app.surface_windows.close_all().unwrap();
    app.poll_remote_live_with(false, &mut config).unwrap();
    fixture::wait(app);
    app.poll_remote_live_with(false, |_| panic!("duplicate inactive request"))
        .unwrap();
    assert!(app.surface_windows.iter_mut().next().is_none());
    assert_eq!(app.surface_windows.retired_records().len(), 2);
}

#[test]
fn application_attach_activates_fetches_and_retires_images_through_tls() {
    run(false, false);
}
#[test]
fn application_attach_revocation_discards_completed_image_without_inactive_claim() {
    run(true, false);
}
#[test]
fn application_rejected_surface_is_deactivated_during_normal_poll() {
    run(false, true);
}
