//! Focus mode of the original TLS/Grant harness; real blur produces change, still emitted explicitly by listeners only.
use super::*;
use windows::{
    core::HSTRING,
    Win32::UI::{Controls::*, Input::KeyboardAndMouse::GetFocus},
};
pub(super) unsafe fn prepare(
    app: &mut Application,
    root: &std::path::Path,
    parent: HWND,
    round: u64,
) {
    // Foreground may be lost between TLS/server batches; acquire it only before each driven segment's send, without replaying failed requests.
    let _foreground = crate::surface_javascript::fixture::focus_foreground(parent);
    let edit = text::child(parent);
    let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(parent));
    assert_eq!(GetFocus(), parent);
    assert_eq!(
        GetForegroundWindow(),
        parent,
        "focus fixture lost foreground before request"
    );
    app.surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_focus_request_for_test(true);
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    wait_js(app, true);
    assert_eq!(
        GetForegroundWindow(),
        parent,
        "focus fixture lost foreground before delivery"
    );
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    assert_eq!(
        GetForegroundWindow(),
        parent,
        "focus fixture lost foreground after delivery"
    );
    assert_eq!(GetFocus(), edit);
    assert_eq!(app.remote_sessions.wait_handles().count(), 0);
    SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
    let value = HSTRING::from(format!("typed:{round}🙂"));
    SendMessageW(
        edit,
        EM_REPLACESEL,
        Some(WPARAM(1)),
        Some(LPARAM(value.as_ptr() as isize)),
    );
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    wait_js(app, true);
    assert_eq!(
        GetForegroundWindow(),
        parent,
        "blur fixture lost foreground before delivery"
    );
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    assert_eq!(
        GetForegroundWindow(),
        parent,
        "blur fixture lost foreground after delivery"
    );
    // blur comes from the real input listener, not system focus loss caused merely by clicking another button.
    assert_eq!(GetFocus(), parent);
    // Later refresh in the same poll may accept a fast change reply and start the original emit batch; do not assume it remains pending.
    assert!(app.remote_sessions.wait_handles().count() <= 1);
}
pub(super) fn updated(
    window: &crate::desktop::surface_authorized_window::AuthorizedWindow,
    parent: HWND,
    edit: HWND,
    round: u64,
) {
    assert_eq!(text::child(parent), edit);
    let mut value = [0u16; 128];
    let length = unsafe { GetWindowTextW(edit, &mut value) };
    assert_eq!(
        String::from_utf16(&value[..length as usize]).unwrap(),
        format!("server:{}", if round == 0 { 45 } else { 48 })
    );
    assert_eq!(
        window.javascript_text_for_test(),
        format!("change|typed:{round}🙂")
    );
    // Focus was checked at prepare's actual delivery boundary; the test does not own user foreground during network batches, so updates validate data only.
}
pub(super) unsafe fn stale(
    app: &mut Application,
    root: &std::path::Path,
    parent: HWND,
    policy: bool,
) {
    // Staleness/policy negatives must also start with valid foreground state to avoid false passes from another rejection condition.
    let _foreground = crate::surface_javascript::fixture::focus_foreground(parent);
    let edit = text::child(parent);
    assert_eq!(GetFocus(), parent);
    assert_eq!(GetForegroundWindow(), parent);
    app.surface_windows
        .iter_mut()
        .next()
        .unwrap()
        .javascript_focus_request_for_test(true);
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    wait_js(app, true);
    assert_eq!(
        GetForegroundWindow(),
        parent,
        "stale focus fixture lost foreground before delivery"
    );
    if policy {
        app.authorization.borrow_mut().busy = true;
    } else {
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(edit));
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(Some(parent));
    }
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    assert_eq!(GetForegroundWindow(), parent);
    assert_ne!(GetFocus(), edit);
    assert_eq!(app.remote_sessions.wait_handles().count(), 0);
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(window.is_alive() && !window.javascript_pending_for_test());
    assert_eq!(window.snapshot_revision(), 1);
    app.authorization.borrow_mut().busy = false;
    app.poll_remote_live_with(true, |_| panic!("focus request replayed"))
        .unwrap();
    assert_ne!(GetFocus(), edit);
}
