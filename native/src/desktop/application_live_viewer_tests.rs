//! 真实Application/菜单队列/TLS/WSS/NOACTIVATE帧与回收；Surface由原合成夹具认证，非真实Loom。
use super::*;
use crate::desktop::device_authorization_ui::{live, Request};
use std::time::{Duration, Instant};
use windows::Win32::System::Threading::WaitForSingleObject;
#[path = "application_live_viewer_art_control_tests.rs"]
mod art_interaction;
#[path = "application_live_viewer_input_tests.rs"]
mod interaction;
#[path = "application_live_viewer_joint_tests.rs"]
mod joint;
#[path = "application_live_viewer_observation_tests.rs"]
mod observation;
#[path = "application_live_viewer_peer.rs"]
mod peer;
unsafe fn spin(
    app: &mut Application,
    root: &std::path::Path,
    ready: impl Fn(&Application) -> bool,
) {
    assert!(spin_result(app, root, ready).is_none());
}
unsafe fn spin_result(
    app: &mut Application,
    root: &std::path::Path,
    ready: impl Fn(&Application) -> bool,
) -> Option<windows::core::Error> {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut failure = None;
    loop {
        if let Err(error) = app.poll_remote_live_with(false, |origin| fixture::config(origin, root))
        {
            assert!(failure.replace(error).is_none(), "duplicate viewer failure");
        }
        if ready(app) {
            return failure;
        }
        assert!(
            Instant::now() < deadline,
            "viewer phase timed out: {}",
            app.authorization.borrow().live.status
        );
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
unsafe fn close_surface(app: &mut Application) {
    let now = crate::desktop::surface_windows::now_ms().unwrap();
    let window = app.surface_windows.iter_mut().next().unwrap();
    window.close().unwrap();
    let mut owner = window.begin_deactivation(now).unwrap().unwrap();
    let revision = owner
        .begin(crate::surface_live_binding::Lifecycle::Inactive)
        .unwrap();
    let binding = owner.binding();
    owner.acknowledge(&serde_json::to_vec(&serde_json::json!({"descriptor":{"instanceId":binding.instance(),"attachmentId":binding.attachment(),"hookNodeId":binding.node(),"deviceId":binding.device()},"lifecycle":"inactive","lifecycleRevision":revision})).unwrap(),crate::surface_live_binding::Lifecycle::Inactive,revision).unwrap();
    assert!(window.finish_activation(owner).is_ok());
}
fn run(mode: u8) {
    run_host(mode, false);
}
fn run_host(mode: u8, local_art: bool) {
    let _javascript = local_art.then(|| crate::surface_javascript::fixture::SERIAL.lock().unwrap());
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = format!(
        "https://127.0.0.1:{}",
        listener.local_addr().unwrap().port()
    );
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let mut peer = if local_art {
        if mode >= 3 {
            peer::Peer::start_art_input(listener, tls, mode)
        } else {
            peer::Peer::start_art(listener, tls, mode == 2)
        }
    } else if mode >= 3 {
        peer::Peer::start_input(listener, tls, mode)
    } else {
        peer::Peer::start(listener, tls, mode == 2)
    };
    unsafe {
        let (_class, mut app) = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew { client, store })
            .unwrap();
        for handle in app.authorization_owner.wait_handles() {
            assert_eq!(
                WaitForSingleObject(HANDLE(handle.as_raw_handle()), 5000),
                WAIT_OBJECT_0
            );
        }
        assert!(matches!(
            app.authorization_owner.poll().as_slice(),
            [crate::device_authorization_owner::Event::Authorized { .. }]
        ));
        let now = crate::desktop::surface_windows::now_ms().unwrap();
        if local_art {
            app.art_issuers = peer::Peer::art_authority(&origin);
            let node = app.documents[0].state.unit_id.to_string();
            app.start_local_art(
                &origin,
                "vendor/viewer".into(),
                &node,
                fixture::config(&origin, files.root()).unwrap(),
            )
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                app.process_local_arts(false);
                if app.local_arts.test_counts() == (1, 1) {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "Art host deadline: {}",
                    app.authorization.borrow().status
                );
                std::thread::sleep(Duration::from_millis(1));
            }
        } else {
            let mut window = fixture::window_with_sources(
                &mut app,
                &origin,
                now,
                vec![
                    serde_json::json!({"id":"label","type":"text","props":{"text":"Viewer host"}}),
                ],
                vec![],
            );
            let hwnd = window.edit_test_parent();
            SetWindowPos(
                hwnd,
                None,
                -10000,
                -10000,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
            .unwrap();
            window.show(now).unwrap();
            assert!(app.surface_windows.insert(window).is_ok());
        }
        app.settle_live_viewers().unwrap();
        assert_eq!(app.authorization.borrow().live.bindings.len(), 1);
        app.authorization.borrow_mut().request = Some(Request::Discover);
        app.process_authorization_with_projection(|origin| fixture::config(origin, files.root()));
        assert!(app.remote_sessions.discovering());
        fixture::wait(&app);
        app.poll_remote_live_with(false, |origin| fixture::config(origin, files.root()))
            .unwrap();
        let selected = {
            let view = app.authorization.borrow();
            live::Selection {
                origin: origin.clone(),
                identity: view.catalog.as_ref().unwrap().entries[0].identity.clone(),
                binding: view.live.bindings[0].clone(),
            }
        };
        live::select(app.documents[0].window.0, &app.documents[0].state, selected).unwrap();
        app.process_authorization_with_projection(|origin| fixture::config(origin, files.root()));
        assert_eq!(app.remote_sessions.viewers().len(), 1);
        let key = app.remote_sessions.viewers().pop().unwrap().key;
        let deadline = Instant::now() + Duration::from_secs(5);
        while !peer.attached.load(std::sync::atomic::Ordering::Acquire) {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        if mode == 2 {
            if local_art {
                app.local_arts.reset();
            } else {
                close_surface(&mut app);
            }
            app.settle_live_viewers().unwrap();
            assert!(app.remote_sessions.viewers()[0].stopping);
            spin(&mut app, files.root(), |app| {
                app.remote_sessions.viewers().is_empty()
            });
            assert_eq!(peer.streams.load(std::sync::atomic::Ordering::Acquire), 0);
        } else {
            spin(&mut app, files.root(), |app| {
                app.remote_sessions.viewers()[0].shown
                    && peer.streams.load(std::sync::atomic::Ordering::Acquire) == 2
            });
            let deadline = Instant::now() + Duration::from_secs(5);
            let (metadata, pixel) = loop {
                if let Some(frame) = app.remote_sessions.test_frame(&key).unwrap() {
                    break frame;
                }
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            };
            assert_eq!(
                (metadata.width, metadata.height, metadata.frame_id),
                (1, 1, 1)
            );
            assert_eq!(pixel, [17, 83, 211, 255]);
            assert_ne!(
                GetForegroundWindow(),
                FindWindowW(w!("Otter.Native.Relay"), None).unwrap()
            );
            if mode >= 3 {
                if mode == 9 {
                    observation::exercise(&mut app, files.root(), &key, &peer);
                } else {
                    interaction::exercise(&mut app, files.root(), &key, &peer, mode);
                }
                peer.finish();
                assert_eq!(app.remote_sessions.wait_handles().count(), 0);
                app.shutdown();
                assert!(app.remote_sessions.finished());
                return;
            }
            // Ready后关闭发起Surface不撤销独立观看；显式停止/撤权才关窗。
            if local_art {
                app.local_arts.reset();
            } else {
                close_surface(&mut app);
            }
            app.settle_live_viewers().unwrap();
            assert!(!app.remote_sessions.viewers()[0].stopping);
            app.authorization.borrow_mut().request = Some(if mode == 0 {
                Request::StopLive(key.clone())
            } else {
                Request::Cancel
            });
            app.process_authorization_with_projection(|origin| {
                fixture::config(origin, files.root())
            });
            assert!(app.remote_sessions.viewers()[0].stopping);
            spin(&mut app, files.root(), |app| {
                app.remote_sessions.viewers().is_empty()
            });
            assert!(app.remote_sessions.test_frame(&key).unwrap().is_none());
        }
        peer.finish();
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        app.shutdown();
        assert!(app.remote_sessions.finished());
    }
}
#[test]
fn menu_join_renders_exact_frame_then_explicit_stop_reaps() {
    run(0);
}
#[test]
fn authorization_cancel_closes_window_and_reaps_without_rejoining() {
    run(1);
}
#[test]
fn closed_surface_before_ready_discards_late_join_and_never_opens_window() {
    run(2);
}
#[test]
fn local_art_menu_join_renders_exact_frame_then_stop_reaps() {
    run_host(0, true);
}
#[test]
fn local_art_authorization_cancel_reaps_independent_viewer() {
    run_host(1, true);
}
#[test]
fn local_art_closed_before_ready_discards_late_join_without_window() {
    run_host(2, true);
}
