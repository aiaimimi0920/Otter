//! EDIT input mode of the original Application harness; local input does not network, and only change emit enters the original two-event batch.
use super::*;
use windows::{
    core::HSTRING,
    Win32::UI::Controls::{EM_REPLACESEL, EM_SETSEL},
};
pub(super) fn child(parent: HWND) -> HWND {
    unsafe { FindWindowExW(Some(parent), None, windows::core::w!("EDIT"), None).unwrap() }
}
fn text(edit: HWND) -> String {
    let mut buffer = [0u16; 128];
    let n = unsafe { GetWindowTextW(edit, &mut buffer) };
    String::from_utf16(&buffer[..n as usize]).unwrap()
}
// The caller owns this thread's Apartment/Application, with the same HWND-message-pump preconditions as the original harness.
pub(super) unsafe fn prepare(
    app: &mut Application,
    root: &std::path::Path,
    parent: HWND,
    round: u64,
) {
    let edit = child(parent);
    unsafe {
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let value = HSTRING::from(format!("typed:{round}🙂"));
        SendMessageW(
            edit,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(value.as_ptr() as isize)),
        );
    }
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    assert_eq!(app.remote_sessions.wait_handles().count(), 0);
    wait_js(app, true);
    app.poll_remote_live_with(true, |o| fixture::config(o, root))
        .unwrap();
    let window = app.surface_windows.iter_mut().next().unwrap();
    assert!(window.local_edit_enabled());
    assert_eq!(
        window.javascript_text_for_test(),
        format!("input|typed:{round}🙂|{}", round * 2 + 1)
    );
    assert_eq!(child(parent), edit);
    assert_eq!(app.remote_sessions.wait_handles().count(), 0);
    unsafe {
        SendMessageW(
            parent,
            crate::desktop::surface_edit_messages::COMMIT,
            None,
            Some(LPARAM(edit.0 as isize)),
        );
    }
}
pub(super) fn updated(
    window: &crate::desktop::surface_authorized_window::AuthorizedWindow,
    parent: HWND,
    edit: HWND,
    round: u64,
) {
    assert_eq!(child(parent), edit);
    assert_eq!(
        text(edit),
        format!("server:{}", if round == 0 { 45 } else { 48 })
    );
    assert_eq!(
        window.javascript_text_for_test(),
        format!("change|typed:{round}🙂|{}", round * 2 + 2)
    );
}
