//! Owned short-status-window closure and synchronous reentrancy; no user-focus stealing, hooks, or input injection.
use super::*;

thread_local! {
    static TEXT_ATTEMPTS: Cell<u32> = const { Cell::new(0) };
}

unsafe extern "system" fn fail_first_text(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_SETTEXT
        && TEXT_ATTEMPTS.with(|count| {
            let previous = count.get();
            count.set(previous + 1);
            previous == 0
        })
    {
        return LRESULT(0);
    }
    procedure(hwnd, msg, wp, lp)
}

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

#[test]
fn status_text_failure_keeps_stop_visible_and_retries_same_title_once() {
    unsafe {
        let window = StatusWindow::new().unwrap();
        let hwnd = window.hwnd;
        let label = "Otter Tile · 等待显示器";
        let title = || {
            let mut text = [0u16; 64];
            let length = GetWindowTextW(hwnd, &mut text) as usize;
            String::from_utf16(&text[..length]).unwrap()
        };
        TEXT_ATTEMPTS.with(|count| count.set(0));
        assert_ne!(
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, fail_first_text as *const () as isize),
            0
        );

        window.show(label);
        assert!(IsWindowVisible(hwnd).as_bool());
        assert!(IsWindowVisible(GetDlgItem(Some(hwnd), 1).unwrap()).as_bool());
        assert_eq!(title(), "Otter Tile");
        window.show(label);
        assert_eq!(title(), label);
        window.show(label);
        assert_eq!(TEXT_ATTEMPTS.with(Cell::get), 2);
    }
}
