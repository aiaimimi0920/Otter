//! 原 Application/TLS harness 的 Enter 模式；定向 EDIT 消息，不用 parent COMMIT 冒充键盘。
use super::*;
use windows::{
    core::HSTRING,
    Win32::UI::{Controls::*, Input::KeyboardAndMouse::*},
};
unsafe fn key(edit: HWND, up: bool) {
    let parent = GetParent(edit).unwrap();
    // 定向 SendMessage 不会建立真实前台；必须在发送前证明产品准入条件，而非事后重放。
    assert_eq!(
        GetForegroundWindow(),
        parent,
        "keyboard fixture lost foreground before send"
    );
    assert_eq!(
        GetFocus(),
        edit,
        "keyboard fixture lost EDIT focus before send"
    );
    assert!(IsWindowEnabled(edit).as_bool() && IsWindowVisible(edit).as_bool());
    SendMessageW(
        edit,
        if up { WM_KEYUP } else { WM_KEYDOWN },
        Some(WPARAM(VK_RETURN.0 as usize)),
        Some(LPARAM(0)),
    );
}
// 原 harness 已持有本线程 Apartment；调用方与其使用同一 HWND owner。
unsafe fn poll(app: &mut Application, root: &std::path::Path) {
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    if app
        .surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_pending_for_test()
    {
        wait_js(app, true);
    }
}
pub(super) unsafe fn prepare(
    app: &mut Application,
    root: &std::path::Path,
    parent: HWND,
    round: u64,
) {
    // TLS/服务端批次期间不假设前台仍归本测试；每段真实键盘驱动独立建立并恢复前台。
    let _foreground = crate::surface_javascript::fixture::focus_foreground(parent);
    let edit = text::child(parent);
    let _ = SetFocus(Some(edit));
    assert_eq!(GetFocus(), edit);
    SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
    let value = HSTRING::from(format!("typed:{round}🙂"));
    SendMessageW(
        edit,
        EM_REPLACESEL,
        Some(WPARAM(1)),
        Some(LPARAM(value.as_ptr() as isize)),
    );
    key(edit, false);
    key(edit, true);
    let canceled = format!("input:typed:{round}🙂>down:cancel:first>up");
    for step in 0..16 {
        poll(app, root);
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        let window = app.surface_windows.iter_mut().next().unwrap();
        assert_eq!(window.snapshot_revision(), round + 1);
        assert!(window.is_alive());
        if !window.javascript_pending_for_test() && window.javascript_text_for_test() == canceled {
            break;
        }
        assert!(step < 15, "canceled Enter did not drain");
    }
    assert_eq!(
        app.surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .javascript_text_for_test(),
        canceled
    );
    // 同一字段下一次 Enter 不取消；直到默认 change listener 的显式 emit 才允许联网。
    key(edit, false);
    for step in 0..16 {
        poll(app, root);
        if app.remote_sessions.wait_handles().count() == 1 {
            break;
        }
        assert!(step < 15, "allowed Enter did not submit its default change");
    }
    assert_eq!(
        app.surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .javascript_text_for_test(),
        format!("{canceled}>down:allow:first>change:typed:{round}🙂")
    );
}
pub(super) fn updated(
    window: &crate::desktop::surface_authorized_window::AuthorizedWindow,
    parent: HWND,
    edit: HWND,
    round: u64,
) {
    assert_eq!(text::child(parent), edit);
    assert_eq!(
        window.javascript_text_for_test(),
        format!("server:{}:{}", if round == 0 { 45 } else { 48 }, round + 1)
    );
    let mut value = [0u16; 128];
    let n = unsafe { GetWindowTextW(edit, &mut value) };
    assert_eq!(
        String::from_utf16(&value[..n as usize]).unwrap(),
        format!("server:{}", if round == 0 { 45 } else { 48 })
    );
}
pub(super) unsafe fn release(app: &mut Application, root: &std::path::Path, parent: HWND) {
    let _foreground = crate::surface_javascript::fixture::focus_foreground(parent);
    let edit = text::child(parent);
    let _ = SetFocus(Some(edit));
    key(edit, true);
    for step in 0..16 {
        poll(app, root);
        assert_eq!(app.remote_sessions.wait_handles().count(), 0);
        let window = app.surface_windows.iter_mut().next().unwrap();
        if !window.javascript_pending_for_test() && window.javascript_text_for_test() == "up" {
            break;
        }
        assert!(
            step < 15,
            "keyup did not drain: focus={:?} edit={edit:?} foreground={:?} parent={parent:?} enabled={} visible={} local_edit={} pending={} text={:?}",
            GetFocus(), GetForegroundWindow(), IsWindowEnabled(edit).as_bool(),
            IsWindowVisible(edit).as_bool(), window.local_edit_enabled(),
            window.javascript_pending_for_test(), window.javascript_text_for_test()
        );
    }
}
