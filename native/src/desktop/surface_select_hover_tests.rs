//! 向自有 HWND 发送局部指针消息；不移动光标、不注入全局输入。
use super::*;
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
unsafe fn enter(hwnd: HWND) {
    SendMessageW(hwnd, WM_MOUSEMOVE, None, Some(LPARAM(5 | (5 << 16))));
}
unsafe fn background(hwnd: HWND, name: &str, color: [u8; 3]) {
    let pixels = pixels(hwnd, name);
    let count = pixels.iter().filter(|p| **p == color).count();
    assert!(
        count > pixels.len() / 2,
        "background mismatch {color:?}: {count}/{}",
        pixels.len()
    );
}
#[test]
fn surface_select_hover_enter_leave_and_disabled_cleanup_preserve_value() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    window.show().unwrap();
    unsafe {
        enter(hwnd);
        background(hwnd, "select-hover.png", [0x2c, 0x22, 0x18]);
        SendMessageW(hwnd, WM_MOUSELEAVE, None, None);
        background(hwnd, "select-leave.png", [0x20, 0x17, 0x11]);
        enter(hwnd);
        window.set_input_policy(false, false).unwrap();
        enter(hwnd);
        background(hwnd, "select-hover-disabled.png", [0x20, 0x17, 0x11]);
        window.set_input_policy(true, true).unwrap();
        background(hwnd, "select-hover-reenabled.png", [0x20, 0x17, 0x11]);
        assert_eq!(index(hwnd), 0);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn surface_select_hover_cancel_hide_and_outside_coordinates_clear_highlight() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    window.show().unwrap();
    unsafe {
        enter(hwnd);
        SendMessageW(hwnd, WM_CANCELMODE, None, None);
        background(hwnd, "select-hover-cancel.png", [0x20, 0x17, 0x11]);
        enter(hwnd);
        let _ = ShowWindow(hwnd, SW_HIDE);
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        background(hwnd, "select-hover-reshown.png", [0x20, 0x17, 0x11]);
        enter(hwnd);
        // 捕获期间可能收到客户区外坐标；低 16 位必须按有符号值解析。
        SendMessageW(hwnd, WM_MOUSEMOVE, None, Some(LPARAM(0xffff | (5 << 16))));
        background(hwnd, "select-hover-outside.png", [0x20, 0x17, 0x11]);
        assert_eq!(index(hwnd), 0);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
