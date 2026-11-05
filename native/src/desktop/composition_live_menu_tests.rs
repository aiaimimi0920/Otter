//! 无框菜单、角命中和反馈的隐藏 HWND 测试；不在测试桌面弹出菜单或对话框。
use super::*;
use crate::{desktop::Apartment, live_frames::Source};
unsafe fn window() -> (Source<Texture>, LiveWindow) {
    let source = Source::new();
    let window = LiveWindow::new_at(
        source.subscribe().unwrap(),
        RECT {
            left: -500,
            top: -500,
            right: -372,
            bottom: -372,
        },
    )
    .unwrap();
    super::super::tests::publish(&source);
    SendMessageW(window.hwnd, WM_PAINT, None, None);
    (source, window)
}
unsafe fn label(hwnd: HWND) -> String {
    let mut value = [0u16; 128];
    let count = GetMenuStringW(
        GetSystemMenu(hwnd, false),
        input::TOGGLE as u32,
        Some(&mut value),
        MF_BYCOMMAND,
    );
    String::from_utf16_lossy(&value[..count as usize])
}
#[test]
fn live_frameless_menu_dispatches_existing_commands_and_rejects_closed_owner() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_source, window) = window();
        let style = WINDOW_STYLE(GetWindowLongPtrW(window.hwnd, GWL_STYLE) as u32);
        assert!(!style.contains(WS_CAPTION));
        assert!(!style.contains(WS_THICKFRAME));
        assert!(style.contains(WS_POPUP | WS_SYSMENU));
        let client = local::client_rect(window.hwnd).unwrap();
        let mut outer = RECT::default();
        GetWindowRect(window.hwnd, &mut outer).unwrap();
        assert_eq!(
            (
                outer.left,
                outer.top,
                outer.right - outer.left,
                outer.bottom - outer.top
            ),
            (
                client.x,
                client.y,
                client.width as i32,
                client.height as i32
            )
        );
        show_with(window.hwnd, &window.state, |_| 0x1fe0);
        assert!(window.take_freeze_request());
        show_with(window.hwnd, &window.state, |_| 0);
        assert!(!window.take_freeze_request());
        window.allow_input().unwrap();
        show_with(window.hwnd, &window.state, |_| {
            local::tests::with_keys(&[VK_MENU], || {
                SendMessageW(
                    window.hwnd,
                    WM_LBUTTONDOWN,
                    None,
                    Some(LPARAM(8 | (8 << 16))),
                );
            });
            local::tests::wheel(window.hwnd, &[VK_MENU], -120, (-436, -436));
            SendMessageW(
                window.hwnd,
                WM_SYSCOMMAND,
                Some(WPARAM(input::TOGGLE)),
                None,
            );
            assert!(window.state.drag.get().is_none());
            assert_eq!(window.opacity(), 1.0);
            assert_ne!(window.take_input_request(), Some(true));
            0
        });
        show_with(window.hwnd, &window.state, |_| {
            DestroyWindow(window.hwnd).unwrap();
            0x1fe0
        });
        assert!(!window.take_freeze_request());
        assert!(!window.state.menu_open.get());
    }
}
#[test]
fn live_yellow_l_corner_moves_but_inner_square_stays_source_owned() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_source, window) = window();
        local::tests::with_keys(&[], || {
            SendMessageW(
                window.hwnd,
                WM_LBUTTONDOWN,
                None,
                Some(LPARAM(1 | (10 << 16))),
            );
        });
        assert!(window.state.drag.get().is_some());
        local::drag::cancel(window.hwnd, &window.state);
        window.input_status(true, false, None);
        local::tests::with_keys(&[], || {
            SendMessageW(
                window.hwnd,
                WM_LBUTTONDOWN,
                None,
                Some(LPARAM(8 | (8 << 16))),
            );
        });
        assert!(window.state.drag.get().is_none());
        assert!(window.pop_input().is_some());
        input::cancel(window.hwnd, &window.state);
        window.input_status(true, false, None);
        SendMessageW(
            window.hwnd,
            WM_RBUTTONDOWN,
            None,
            Some(LPARAM(8 | (8 << 16))),
        );
        SendMessageW(window.hwnd, WM_RBUTTONUP, None, Some(LPARAM(8 | (8 << 16))));
        assert!(window.pop_input().is_some());
        assert!(window.pop_input().is_some());
        SendMessageW(
            window.hwnd,
            WM_RBUTTONDOWN,
            None,
            Some(LPARAM(1 | (10 << 16))),
        );
        assert_eq!(window.take_input_request(), Some(false));
        assert!(window.pop_input().is_none());
        assert_ne!(GetCapture(), window.hwnd);
    }
}
#[test]
fn live_frameless_alt_delete_uses_recycle_close_and_failure_notice_is_deduplicated() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let (_source, window) = window();
        window.allow_input().unwrap();
        window.input_status(false, true, None);
        assert!(label(window.hwnd).contains("启用中"));
        window.input_status(true, false, None);
        assert_eq!(label(window.hwnd), "停止控制源窗口");
        window.input_status(false, false, None);
        window.input_failure();
        assert!(label(window.hwnd).contains("上次失败"));
        assert!(window.take_input_notice());
        window.input_failure();
        assert!(!window.take_input_notice());
        window.input_status(false, true, None);
        assert!(!window.take_input_notice());
        window.input_failure();
        assert!(window.take_input_notice());
        window.enable_recycle_close();
        local::tests::with_keys(&[VK_MENU], || {
            SendMessageW(
                window.hwnd,
                WM_SYSKEYDOWN,
                Some(WPARAM(VK_DELETE.0 as usize)),
                None,
            );
        });
        assert!(window.is_alive());
        assert!(window.take_recycle_request());
    }
}
