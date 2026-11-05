//! 原Application/TLS/WSS→菜单→真正只读EDIT及更新/撤权；不代表普通双EXE或Source UIA。
use super::*;
use crate::desktop::composition::relay::{sessions::ViewerKey, window::observation::OPEN};
use std::sync::{atomic::Ordering, Arc};
unsafe fn text(hwnd: HWND) -> String {
    let length = GetWindowTextLengthW(hwnd) as usize;
    assert!(length <= 128 * 1024);
    let mut buffer = vec![0; length + 1];
    let count = GetWindowTextW(hwnd, &mut buffer);
    String::from_utf16(&buffer[..count as usize]).unwrap()
}
pub(super) unsafe fn exercise(
    app: &mut Application,
    root: &std::path::Path,
    key: &Arc<ViewerKey>,
    peer: &peer::Peer,
) {
    let before = app.documents[0]
        .state
        .canvas
        .borrow()
        .editor
        .document_stamp();
    let (viewer, _) = app.remote_sessions.test_viewer_input(key).unwrap();
    SetWindowPos(
        viewer,
        None,
        -10000,
        -10000,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
    )
    .unwrap();
    SendMessageW(viewer, WM_SYSCOMMAND, Some(WPARAM(OPEN)), Some(LPARAM(0)));
    let (window, edit) = app.remote_sessions.test_viewer_observation(key).unwrap();
    assert_ne!(GetForegroundWindow(), window);
    let initial = text(edit);
    assert!(initial.contains("观察：稳定 · 1 项 · 触发注册 1 项"));
    assert!(initial.contains("值：{\"progress\":7}"));
    assert!(initial.contains("稳定 500 ms"));
    assert!(initial.contains("目标：instance:test / attachment:test / label / click / run"));
    assert!(initial.contains("binding:progress · 派发中"));
    assert!(!initial.contains("请求："));
    assert_ne!(
        GetWindowLongPtrW(edit, GWL_STYLE) as u32 & ES_READONLY as u32,
        0
    );
    let mut affinity = 0;
    GetWindowDisplayAffinity(window, &mut affinity).unwrap();
    assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
    peer.semantic.store(1, Ordering::Release);
    spin(app, root, |app| {
        app.remote_sessions
            .test_viewer_observation(key)
            .is_some_and(|(_, e)| text(e).contains("值：{\"progress\":9}"))
    });
    let updated = text(edit);
    assert!(!updated.contains("值：{\"progress\":7}"));
    assert!(updated.contains("最近 1 / 1 项"));
    assert!(updated.contains("binding:progress · 已触发"));
    assert!(updated.contains("请求：action:123"));
    assert!(!updated.contains("surface_action_dispatch_reserved"));
    SendMessageW(window, WM_CLOSE, Some(WPARAM(0)), Some(LPARAM(0)));
    assert!(!IsWindow(Some(window)).as_bool());
    app.poll_remote_live_with(false, |origin| fixture::config(origin, root))
        .unwrap();
    assert!(app.remote_sessions.test_viewer_observation(key).is_none());
    assert_eq!(
        app.remote_sessions.test_frame(key).unwrap().unwrap().1,
        [17, 83, 211, 255]
    );
    SendMessageW(viewer, WM_SYSCOMMAND, Some(WPARAM(OPEN)), Some(LPARAM(0)));
    let (reopened, reedit) = app.remote_sessions.test_viewer_observation(key).unwrap();
    assert!(text(reedit).contains("值：{\"progress\":9}"));
    app.authorization.borrow_mut().request = Some(Request::Cancel);
    app.process_authorization_with_projection(|origin| fixture::config(origin, root));
    spin(app, root, |app| app.remote_sessions.viewers().is_empty());
    assert!(!IsWindow(Some(reopened)).as_bool());
    assert!(!IsWindow(Some(reedit)).as_bool());
    assert!(before.matches(&app.documents[0].state.canvas.borrow().editor));
    assert_eq!(peer.controls.load(Ordering::Acquire), 0);
    assert!(peer.inputs.lock().unwrap().is_empty());
}
#[test]
fn surface_viewer_observation_menu_displays_snapshot_delta_audit_then_reaps_on_revoke() {
    run(9);
}
#[test]
fn local_art_viewer_observation_menu_uses_same_authorized_stream_without_executing_actions() {
    run_host(9, true);
}
