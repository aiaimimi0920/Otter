//! 原菜单/原Application/真实TLS/WSS/自有窗口消息闭环；不向用户窗口发送输入，不代表物理跨端。
use super::*;
use crate::desktop::composition::relay::{
    sessions::ViewerKey,
    window::input::{ACQUIRE, RELEASE},
};
use std::sync::{atomic::Ordering, Arc};
pub(super) unsafe fn exercise(
    app: &mut Application,
    root: &std::path::Path,
    key: &Arc<ViewerKey>,
    peer: &peer::Peer,
    mode: u8,
) {
    let before = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    let (hwnd, status) = app.remote_sessions.test_viewer_input(key).unwrap();
    assert!(status.available);
    assert!(!status.enabled);
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
    SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(ACQUIRE)), Some(LPARAM(0)));
    spin(app, root, |app| {
        app.remote_sessions
            .test_viewer_input(key)
            .is_some_and(|(_, s)| s.enabled)
    });
    SendMessageW(hwnd, WM_KEYDOWN, Some(WPARAM(65)), Some(LPARAM(0)));
    if mode != 7 {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect).unwrap();
        let point =
            LPARAM((((rect.bottom / 2) as usize) << 16 | (rect.right / 2) as usize) as isize);
        SendMessageW(hwnd, WM_LBUTTONDOWN, Some(WPARAM(0)), Some(point));
    }
    if mode == 7 {
        let error = spin_result(app, root, |app| app.remote_sessions.viewers().is_empty())
            .expect("input HTTP failure must be reported");
        assert_eq!(error.code(), E_FAIL);
        assert!(error.message().contains("Http(Status(500))"));
        assert_eq!(peer.inputs.lock().unwrap().len(), 1);
    } else {
        spin(app, root, |_| peer.inputs.lock().unwrap().len() == 2);
    }
    let inputs = peer.inputs.lock().unwrap().clone();
    assert_eq!(
        inputs[0],
        serde_json::json!({"kind":"key","data":{"code":"vk:65","state":"pressed","modifiers":0}})
    );
    if mode != 7 {
        assert_eq!(inputs[1]["kind"], "mouse_button");
        assert_eq!(inputs[1]["data"]["button"], "left");
        assert_eq!(inputs[1]["data"]["state"], "pressed");
        assert_eq!(inputs[1]["data"]["clickCount"], 1);
        let x = inputs[1]["data"]["x"].as_f64().unwrap();
        let y = inputs[1]["data"]["y"].as_f64().unwrap();
        assert!((x - 0.5).abs() < 0.002 && (y - 0.5).abs() < 0.002);
        match mode {
            3 => {
                SendMessageW(hwnd, WM_SYSCOMMAND, Some(WPARAM(RELEASE)), Some(LPARAM(0)));
            }
            4 => {
                SendMessageW(hwnd, WM_KILLFOCUS, Some(WPARAM(0)), Some(LPARAM(0)));
            }
            5 => {
                assert!(app.remote_sessions.stop_viewer(key));
            }
            6 => {}
            8 => app.local_arts.reset(),
            10 => {
                SendMessageW(
                    hwnd,
                    WM_SYSCOMMAND,
                    Some(WPARAM(
                        crate::desktop::composition::relay::window::observation::OPEN,
                    )),
                    Some(LPARAM(0)),
                );
                assert!(app.remote_sessions.test_viewer_observation(key).is_some());
            }
            _ => unreachable!(),
        }
        if mode == 6 {
            spin(app, root, |app| {
                app.remote_sessions
                    .test_viewer_input(key)
                    .is_some_and(|(_, s)| !s.enabled && !s.pending)
            });
            assert_eq!(peer.controls.load(Ordering::Acquire), 1);
        } else {
            spin(app, root, |_| peer.controls.load(Ordering::Acquire) == 2);
        }
        if mode == 8 {
            spin(app, root, |app| {
                app.remote_sessions
                    .test_viewer_input(key)
                    .is_some_and(|(_, s)| !s.available && !s.enabled && !s.pending)
            });
            assert_eq!(
                app.remote_sessions.test_frame(key).unwrap().unwrap().1,
                [17, 83, 211, 255]
            );
        }
        if mode != 5 {
            assert!(app.remote_sessions.test_viewer_input(key).is_some());
            assert!(app.remote_sessions.stop_viewer(key));
        }
        spin(app, root, |app| app.remote_sessions.viewers().is_empty());
    }
    assert_eq!(app.documents.len(), 1);
    assert!(before.matches(&app.documents[0].state.canvas.borrow().editor));
    assert!(app.remote_sessions.test_frame(key).unwrap().is_none());
}
#[test]
fn surface_viewer_menu_control_release_keeps_viewing_until_stop() {
    run(3);
}
#[test]
fn surface_viewer_focus_loss_releases_control_without_closing_view() {
    run(4);
}
#[test]
fn surface_viewer_close_releases_held_input_control_before_reaping() {
    run(5);
}
#[test]
fn surface_viewer_remote_revocation_disables_input_but_preserves_view() {
    run(6);
}
#[test]
fn surface_viewer_input_http_failure_closes_without_replaying_or_editing_documents() {
    run(7);
}
#[test]
fn observation_menu_releases_remote_input_and_stop_destroys_owned_inspector() {
    run(10);
}
