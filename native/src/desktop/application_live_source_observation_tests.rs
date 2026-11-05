//! 实际菜单/Application→受限MTA/UIA→原TLS发布/稳定/变更/停止；不读取用户窗口。
use super::*;
use crate::desktop::composition::live::window::publication::PUBLISH_OBSERVE;
use crate::live_source_uia_test_window::Window;
fn run(mode: u8) {
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let mut peer = peer::Peer::observation(listener, tls);
    unsafe {
        let target = Window::new();
        let (_class, mut app) = fixture::application();
        app.authorization.borrow_mut().origin = Some(origin.clone());
        app.authorization_owner
            .start(crate::device_authorization_runtime::Task::Renew { client, store })
            .unwrap();
        for handle in app.authorization_owner.wait_handles() {
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
        let active = capture(&mut app);
        app.live_sessions[0]
            .test_input_target(crate::live_window_target::Target::bind(target.hwnd).unwrap());
        let identity = app.live_sessions[0].input_identity();
        let hwnd = app.live_sessions[0].group_dialog_owner().unwrap();
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(PUBLISH_OBSERVE)), None);
        spin(&mut app, files.root(), |_| {
            peer.observations
                .lock()
                .unwrap()
                .iter()
                .any(|v| v["state"] == "stable" && v["value"]["value"]["text"] == "value-seven")
        });
        assert!(peer.received.load(Ordering::Acquire));
        let values = peer.observations.lock().unwrap().clone();
        assert!(!serde_json::to_string(&values)
            .unwrap()
            .contains("password-secret"));
        if mode == 0 {
            target.value("value-eight");
            spin(&mut app, files.root(), |_| {
                peer.observations
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|v| v["state"] == "stable" && v["value"]["value"]["text"] == "value-eight")
            });
            SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(STOP)), None);
        } else if mode == 1 {
            app.authorization_owner.invalidate(&origin).unwrap();
        } else {
            app.workspace = crate::workspace::Workspace::default();
        }
        spin(&mut app, files.root(), |app| {
            app.remote_sessions.source_status(&identity).is_none()
        });
        if mode != 1 {
            assert!(peer.closed.load(Ordering::Acquire));
            assert!(app.authorization.borrow().live.status.contains("Confirmed"));
        }
        assert!(app.live_sessions[0].is_alive());
        assert_eq!(active.load(Ordering::Acquire), 1);
        let count = peer.observations.lock().unwrap().len();
        app.poll_live_sources_with(&mut |o| fixture::config(o, files.root()));
        assert_eq!(peer.observations.lock().unwrap().len(), count);
        peer.finish();
        app.shutdown_live().unwrap();
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
}
#[test]
fn source_observation_menu_stable_change_and_explicit_stop_preserve_capture() {
    run(0);
}
#[test]
fn source_observation_revoke_cancels_sampler_and_late_publish() {
    run(1);
}
#[test]
fn source_observation_workspace_change_drains_sampler_and_closes_original() {
    run(2);
}
