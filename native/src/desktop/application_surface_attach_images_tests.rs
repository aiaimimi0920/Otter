//! Actual Application attach/activation/resource/close chain; no fabricated Outcome or ACK.
use super::*;
#[path = "application_surface_attach_images_peer.rs"]
mod peer;

fn run(revoke_ready: bool) {
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, server, pem) = fixture_with_pem();
    let origin = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let peer = std::thread::spawn(move || peer::serve(listener, server, revoke_ready));
    unsafe {
        let (_class, mut app) = fixture::application();
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
        app.poll_remote_live_with(false, &mut config).unwrap();
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
            // 非空 resourceId 优先且不回退 src；缺少租约的节点不进入可见投影。
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
            fixture::wait(&app);
            app.poll_remote_live_with(false, |_| panic!("inactive image read configuration"))
                .unwrap();
        }
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert!(!window.is_alive() && !IsWindow(Some(parent)).as_bool());
        assert_eq!(window.installed_images_for_test(), 0);
        let owner = window.lifecycle().unwrap();
        assert!(!owner.disposed());
        assert_eq!(
            owner.binding().lifecycle(),
            if revoke_ready {
                crate::surface_live_binding::Lifecycle::Active
            } else {
                crate::surface_live_binding::Lifecycle::Inactive
            }
        );
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        let listener = peer.join().unwrap();
        assert!(matches!(listener.accept(), Err(e) if e.kind() == std::io::ErrorKind::WouldBlock));
        app.authorization_owner.close();
        app.remote_sessions.close();
        assert!(app.remote_sessions.poll_surfaces().is_empty());
        assert!(app.remote_sessions.finished());
    }
}
#[test]
fn application_attach_activates_fetches_and_retires_images_through_tls() {
    run(false);
}
#[test]
fn application_attach_revocation_discards_completed_image_without_inactive_claim() {
    run(true);
}
