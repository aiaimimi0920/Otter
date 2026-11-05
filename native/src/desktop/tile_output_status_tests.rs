//! 自有短状态窗的关闭与同步重入；不抢用户焦点、不装hook或注入输入。
use super::*;

unsafe extern "system" fn close_on_text(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_SETTEXT {
        let _ = DestroyWindow(hwnd);
        LRESULT(1)
    } else {
        procedure(hwnd, msg, wp, lp)
    }
}
unsafe extern "system" fn close_on_show(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_SHOWWINDOW && wp.0 != 0 {
        let _ = DestroyWindow(hwnd);
        LRESULT(0)
    } else {
        procedure(hwnd, msg, wp, lp)
    }
}

#[test]
fn status_button_and_close_are_terminal_and_do_not_post_process_quit() {
    unsafe {
        for (message, command) in [(WM_COMMAND, 1), (WM_CLOSE, 0)] {
            let window = StatusWindow::new().unwrap();
            assert!(!window.stopped());
            window.show("Otter Tile · 等待显示器");
            assert!(IsWindowVisible(window.hwnd).as_bool());
            SendMessageW(window.hwnd, message, Some(WPARAM(command)), Some(LPARAM(0)));
            assert!(window.stopped());
            assert!(!IsWindow(Some(window.hwnd)).as_bool());
            window.show("Otter Tile · 正在回收输出");
            window.hide();
            assert!(window.stopped());
        }
        let mut message = MSG::default();
        assert!(!PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_NOREMOVE).as_bool());
    }
}

#[test]
fn status_text_and_show_reentrant_close_never_reuse_destroyed_window() {
    unsafe {
        for procedure in [close_on_text as *const (), close_on_show as *const ()] {
            let window = StatusWindow::new().unwrap();
            assert_ne!(
                SetWindowLongPtrW(window.hwnd, GWLP_WNDPROC, procedure as isize),
                0
            );
            window.show("Otter Tile · 等待显示器");
            assert!(window.stopped());
            assert!(!IsWindow(Some(window.hwnd)).as_bool());
            window.show("Otter Tile · 正在回收输出");
            window.hide();
        }
    }
}
