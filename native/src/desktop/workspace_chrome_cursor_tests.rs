//! Endpoints/links share arrow-class cursors; owned HWNDs only, without system-mouse movement.
use super::*;
#[test]
fn endpoint_cursor_remains_arrow_after_press_release_cancel_and_close() {
    unsafe {
        let mut app = visible_app();
        let ui = app.chrome.ui.clone();
        let (strip, _, point) = port(&ui, 0, Side::Output);
        let arrow = LoadCursorW(None, IDC_ARROW).unwrap();
        assert_eq!(GetClassLongPtrW(strip, GCLP_HCURSOR), arrow.0 as usize);
        let lp = LPARAM((HTCLIENT | (WM_MOUSEMOVE << 16)) as isize);
        for cancel in [false, true] {
            mouse(&ui, strip, WM_LBUTTONDOWN, point);
            assert!(ui.press.borrow().is_some());
            if cancel {
                ui.cancel();
            } else {
                mouse(&ui, strip, WM_LBUTTONUP, point);
            }
            assert!(ui.press.borrow().is_none());
            assert_ne!(GetCapture(), strip);
            // DefWindowProc's return value does not identify cursor shape; verify the actual cursor handle.
            SendMessageW(
                strip,
                WM_SETCURSOR,
                Some(WPARAM(strip.0 as usize)),
                Some(lp),
            );
            assert_eq!(GetCursor(), arrow);
        }
        app.shutdown();
        assert!(!IsWindow(Some(strip)).as_bool());
    }
}
