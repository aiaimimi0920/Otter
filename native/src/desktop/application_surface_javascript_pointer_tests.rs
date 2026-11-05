//! Canvas 原生 Pointer 沿原 Application/TLS/Grant/batch 排程，两轮更新不换 child。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
pub(super) unsafe fn prepare(app: &mut Application, root: &std::path::Path, parent: HWND) {
    let _foreground = crate::surface_javascript::fixture::focus_foreground(parent);
    for (message, offset, label) in [
        (WM_LBUTTONDOWN, 0, "down"),
        (WM_MOUSEMOVE, 16, "move"),
        (WM_LBUTTONUP, 16, ""),
    ] {
        app.surface_windows
            .iter_mut()
            .next()
            .unwrap()
            .javascript_pointer_message_for_test(message, offset);
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
            assert!(window.is_alive(), "pointer listener closed window");
            if message == WM_LBUTTONUP {
                if app.remote_sessions.wait_handles().count() == 1 {
                    break;
                }
            } else {
                assert_eq!(app.remote_sessions.wait_handles().count(), 0);
                if !window.javascript_pending_for_test()
                    && window.javascript_text_for_test() == label
                {
                    assert_eq!(GetCapture(), parent);
                    break;
                }
            }
            assert!(step < 15, "pointer edge did not drain");
        }
    }
    assert_ne!(GetCapture(), parent);
}
#[test]
fn application_javascript_pointer_canvas_real_drag_tls_grant_fifo_same_child() {
    run(19);
}
