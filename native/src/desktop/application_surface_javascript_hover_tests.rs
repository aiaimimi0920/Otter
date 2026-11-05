//! 无按键 HWND hover/ancestor leave 沿原 Application/TLS/Grant/batch，两轮不换 child。
use super::*;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, GetFocus};
pub(super) unsafe fn prepare(app: &mut Application, root: &std::path::Path, parent: HWND) {
    let _foreground = crate::surface_javascript::fixture::focus_foreground(parent);
    for message in [WM_MOUSEMOVE, WM_MOUSELEAVE] {
        app.surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .javascript_hover_message_for_test(message);
        assert_eq!(GetFocus(), parent);
        assert_eq!(GetCapture(), HWND::default());
        for step in 0..16 {
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
            let window = app.surface_windows.iter_mut().next().unwrap();
            assert!(window.is_alive(), "hover listener closed window");
            if message == WM_MOUSELEAVE {
                if app.remote_sessions.wait_handles().count() == 1 {
                    break;
                }
            } else {
                assert_eq!(app.remote_sessions.wait_handles().count(), 0);
                if !window.javascript_pending_for_test()
                    && window.javascript_text_for_test() == "hover"
                {
                    break;
                }
            }
            assert!(
                step < 15,
                "hover edge {message} did not drain: text={:?}, pending={}, handles={}",
                window.javascript_text_for_test(),
                window.javascript_pending_for_test(),
                app.remote_sessions.wait_handles().count()
            );
        }
    }
}
#[test]
fn application_javascript_hover_ancestor_leave_tls_grant_fifo_same_child() {
    run(20);
}
