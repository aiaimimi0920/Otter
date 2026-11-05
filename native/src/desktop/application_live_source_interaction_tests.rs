//! 实际原菜单/TLS事件/同一Session worker到自有窗口，核权限、切换、释放和生命周期。
use super::*;
use crate::desktop::composition::live::window::publication::PUBLISH_CONTROL;
use serde_json::{json, Value};
#[path = "application_live_source_input_oracle.rs"]
pub(super) mod oracle;
fn state(revision: u64, controller: Value) -> (&'static str, Value) {
    (
        "session_state",
        json!({"revision":revision,"visibility":"visible","viewers":["a","b"],"controllerDeviceId":controller}),
    )
}
fn input(actor: &str, sequence: u64, kind: Value) -> (&'static str, Value) {
    (
        "input_event",
        json!({"inputSequence":sequence,"issuedAtMs":1,"sourceDeviceId":actor,"kind":kind}),
    )
}
unsafe fn pump(
    app: &mut Application,
    root: &std::path::Path,
    ready: impl Fn(&Application) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.poll_remote_live_with(false, |origin| fixture::config(origin, root))
            .unwrap();
        // 原Application拥有Session一次性终态；不能丢弃poll返回值后把已回收owner留在Vec。
        app.process_live(false).unwrap();
        if ready(app) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{}",
            app.authorization.borrow().live.status
        );
        let mut message = MSG::default();
        while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn run(mode: u8) {
    let _com = Apartment::new().unwrap();
    let (files, store, _) = Files::new();
    let (listener, client, tls, pem) =
        crate::device_session_client::tests::fixture_with_host("127.0.0.1");
    let origin = client.origin().to_owned();
    std::fs::write(files.root().join("fixture-ca.pem"), pem).unwrap();
    let first = vec![
        state(2, json!("a")),
        input(
            "a",
            1,
            json!({"kind":"key","data":{"code":"vk:65","state":"pressed","modifiers":0}}),
        ),
        input(
            "a",
            2,
            json!({"kind":"mouse_button","data":{"x":0.5,"y":0.5,"button":"left","state":"pressed","clickCount":1}}),
        ),
    ];
    let batches = if mode == 1 {
        let mut invalid = first;
        invalid.push(input("b", 1, json!({"kind":"cancel"})));
        vec![invalid]
    } else if mode == 0 {
        vec![
            first,
            vec![
                state(3, json!("b")),
                input(
                    "b",
                    1,
                    json!({"kind":"key","data":{"code":"vk:66","state":"pressed","modifiers":0}}),
                ),
            ],
        ]
    } else {
        vec![first]
    };
    let mut peer = peer::Peer::input(listener, tls, batches);
    unsafe {
        let source = oracle::Window::new();
        let target = crate::live_window_target::Target::bind(source.hwnd).unwrap();
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
        app.live_sessions[0].test_input_target(target);
        let identity = app.live_sessions[0].input_identity();
        let hwnd = app.live_sessions[0].group_dialog_owner().unwrap();
        SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(PUBLISH_CONTROL)), None);
        if mode == 1 {
            pump(&mut app, files.root(), |app| {
                peer.created.load(Ordering::Acquire)
                    && app.remote_sessions.source_status(&identity).is_none()
            });
            assert!(source.messages().is_empty());
        } else {
            let key = if mode == 0 { 66 } else { 65 };
            pump(&mut app, files.root(), |_| {
                let messages = source.messages();
                peer.received.load(Ordering::Acquire)
                    && messages
                        .iter()
                        .any(|&(m, k, _)| m == WM_KEYDOWN && k == key)
                    && messages.iter().any(|&(m, _, _)| m == WM_LBUTTONDOWN)
                    && (mode != 6 || peer.polls.load(Ordering::Acquire) >= 2)
            });
            assert!(app.live_sessions[0].enable_input().is_err());
            if mode == 0 {
                let messages = source.messages();
                assert!(messages.iter().any(|&(m, k, _)| m == WM_KEYUP && k == 65));
                assert!(messages.iter().any(|&(m, _, _)| m == WM_LBUTTONUP));
            }
            match mode {
                2 => app.authorization_owner.invalidate(&origin).unwrap(),
                3 => app.workspace = crate::workspace::Workspace::default(),
                4 => app.live_sessions[0].stop(),
                5 => {
                    app.shutdown_live().unwrap();
                }
                6 => app.live_sessions[0].test_cancel_remote_input(),
                _ => {
                    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(STOP)), None);
                }
            }
            if mode != 5 {
                pump(&mut app, files.root(), |app| {
                    app.remote_sessions.source_status(&identity).is_none()
                        && app
                            .live_sessions
                            .first()
                            .is_none_or(|source| source.remote_input_idle())
                });
            }
            let messages = source.messages();
            assert!(messages.iter().any(|&(m, k, _)| m == WM_KEYUP && k == key));
            assert!(messages.iter().any(|&(m, _, _)| m == WM_LBUTTONUP));
            if mode == 6 {
                assert!(peer.closed.load(Ordering::Acquire));
                assert!(app.live_sessions[0].is_alive());
                assert!(app.authorization.borrow().live.status.contains("Confirmed"));
            }
        }
        assert_ne!(GetForegroundWindow(), source.hwnd);
        peer.finish();
        app.shutdown_live().unwrap();
        assert_eq!(active.load(Ordering::Acquire), 0);
    }
}
#[test]
fn interactive_source_controller_switch_stops_old_held_inputs_and_preserves_capture() {
    run(0)
}
#[test]
fn interactive_source_invalid_batch_applies_no_partial_input() {
    run(1)
}
#[test]
fn interactive_source_revocation_releases_original_worker_inputs() {
    run(2)
}
#[test]
fn interactive_source_workspace_change_releases_original_worker_inputs() {
    run(3)
}
#[test]
fn interactive_source_capture_close_releases_original_worker_inputs() {
    run(4)
}
#[test]
fn interactive_source_application_exit_pumps_input_cleanup() {
    run(5)
}
#[test]
fn interactive_source_idle_bridge_cancel_closes_publication_and_releases_inputs() {
    run(6)
}
