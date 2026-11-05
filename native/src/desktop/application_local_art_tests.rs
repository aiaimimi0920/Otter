//! 真实隐藏 Application 消费 OpenArt 时复核宿主和设备授权，不发起旧远端 attach。
use super::*;
use crate::{desktop::device_authorization_ui::Request, device_authorization_config::Config};
const ORIGIN: &str = "https://loom.example";
#[path = "application_local_art_cache_tests.rs"]
mod cache_tests;
#[path = "application_local_art_images_tests.rs"]
mod images_tests;
#[path = "application_local_art_live_source_tests.rs"]
mod live_source_tests;
#[path = "application_local_art_node_tests.rs"]
mod node_tests;
#[path = "application_local_art_offline_tests.rs"]
mod offline_tests;
#[path = "application_local_art_package_tests.rs"]
mod package_tests;
#[path = "application_local_art_peer.rs"]
mod peer;
#[path = "application_local_art_ports_tests.rs"]
mod ports_tests;
#[path = "application_local_art_result_tests.rs"]
mod result_tests;
#[path = "application_local_art_trigger_tests.rs"]
mod trigger_tests;
#[path = "application_local_art_viewer_tests.rs"]
mod viewer_tests;
fn config(origin: &str) -> std::result::Result<Config, crate::device_authorization_config::Error> {
    Config::from_lookup(origin, |key| match key {
        "OTTER_NATIVE_PROFILE_DIR" => Some(std::env::temp_dir().into_os_string()),
        _ => None,
    })
}
#[test]
fn queued_open_art_requires_current_host_and_device_authorization() {
    unsafe {
        let _environment = environment();
        let mut app = app();
        let node = app.documents[0].state.unit_id.to_string();
        for (node, expected) in [
            ("missing-node".to_owned(), "Art 发起节点已关闭或被替换"),
            (node, "请先获取有效设备授权"),
        ] {
            {
                let mut view = app.authorization.borrow_mut();
                view.closed = false;
                view.origin = Some(ORIGIN.into());
                view.request = Some(Request::OpenArt {
                    art: "vendor/demo".into(),
                    node,
                });
            }
            app.process_authorization_with_projection(config);
            let view = app.authorization.borrow();
            assert!(view.request.is_none());
            assert_eq!(view.status, expected);
            assert!(!view.mount_busy);
            assert!(!app.remote_sessions.attaching());
        }
    }
}

#[test]
fn authenticated_local_art_host_changes_reclaim_hidden_window() {
    let _javascript = crate::surface_javascript::fixture::SERIAL.lock().unwrap();
    unsafe {
        let _environment = environment();
        for change in [0, 1, 2, 3, 4, 5] {
            let mut app = if change == 5 { image_app() } else { app() };
            let peer = peer::Peer::new();
            peer.authorize(&mut app.authorization_owner);
            let node = app.documents[0].state.unit_id.to_string();
            assert!(app
                .start_local_art(
                    &peer.origin,
                    "vendor/application".into(),
                    &node,
                    peer.config()
                )
                .is_err());
            assert!(!app.local_arts.busy());
            app.art_issuers = peer.authority();
            app.start_local_art(
                &peer.origin,
                "vendor/application".into(),
                &node,
                peer.config(),
            )
            .unwrap();
            settle_local(&mut app, (1, 1));
            assert!(!app.remote_sessions.attaching());
            assert!(!app.authorization.borrow().mount_busy);
            peer.finish();
            match change {
                0 => app.documents[0]
                    .state
                    .accept_upstream
                    .set(!app.documents[0].state.accept_upstream.get()),
                1 => app.documents[0].state.group_locked.set(true),
                2 => {
                    DestroyWindow(app.documents[0].window.0).unwrap();
                    app.collect_closed();
                }
                3 => restore_empty(&mut app),
                4 => {
                    app.shutdown();
                    assert!(app.authorization.borrow().closed);
                    assert!(app.authorization_owner.finished());
                }
                _ => recycle_last(&mut app),
            }
            settle_local(&mut app, (0, 0));
            assert!(!app.authorization.borrow().mount_busy);
        }
    }
}
unsafe fn image_app() -> Application {
    let instance: HINSTANCE = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)
        .unwrap()
        .into();
    let image = crate::desktop::image::Image::from_encoded(
        crate::desktop::application::smoke::image_fixture::PNG,
    )
    .unwrap();
    let document =
        DocumentWindow::create(instance, State::new(Some(image), false).unwrap(), false).unwrap();
    Application::new(instance, document)
}
unsafe fn restore_empty(app: &mut Application) {
    let old = app.documents[0].window.0;
    let identity = app.workspace.identity_stamp();
    app.request_restore_input(crate::desktop::session_job::Input::Bytes(
        br#"{"documentSchemaVersion":1,"documentRevision":1,"otterNativeCoordinateSpace":{"version":1,"origin":[0,0],"scale":1},"stickers":[],"links":[]}"#.to_vec()), None, None).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while app.restore_job.is_some() {
        app.process_restore(false).unwrap();
        assert!(Instant::now() < deadline, "local Art restore deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!IsWindow(Some(old)).as_bool());
    assert!(!identity.matches(&app.workspace));
    assert_eq!(app.documents.len(), 1);
    assert!(app.documents[0].is_alive());
}
unsafe fn recycle_last(app: &mut Application) {
    let old = app.documents[0].window.0;
    let old_unit = app.documents[0].state.unit_id.clone();
    app.sync_frozen_menu();
    crate::desktop::context_menu::execute(
        old,
        &app.documents[0].state,
        crate::desktop::reference_ui::RECYCLE,
    )
    .unwrap();
    app.process_reference(false).unwrap();
    assert!(
        app.save_job.is_some(),
        "recycle must enter its real background preparation"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.save_job.is_some() {
        app.process_reference(false).unwrap();
        assert!(Instant::now() < deadline, "local Art recycle deadline");
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(!IsWindow(Some(old)).as_bool());
    assert!(!app.workspace.contains(&old_unit));
    assert_eq!(app.documents.len(), 1);
    assert!(app.documents[0].is_alive());
    assert_ne!(app.documents[0].state.unit_id, old_unit);
    assert_eq!(
        crate::session_library::decode(
            &app.restored_session.as_ref().unwrap().original,
            crate::session_library::Kind::RecycleBin
        )
        .unwrap()
        .entries()
        .len(),
        1
    );
}
fn settle_local(app: &mut Application, expected: (usize, usize)) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.process_local_arts(false);
        if app.local_arts.test_counts() == expected {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "local Art did not settle: {}",
            app.authorization.borrow().status
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
