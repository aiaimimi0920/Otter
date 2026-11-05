//! 上下行选择走正式消息、Host 与自动显露；线程键状态在退出时恢复。
use super::*;
fn multiline() -> SurfaceWindow {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":45},"children":[{"id":"text","type":"text","props":{"text":"WWWW\nI\nWWWW\nI\nWWWW","selectable":true},"style":{"fontSize":20,"whiteSpace":"pre"},"layout":{"width":100,"height":200,"minHeight":200}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.set_input_policy(true, false).unwrap();
    unsafe {
        keyboard::focus_node(window.hwnd, &window.state, "text");
    }
    window
        .state
        .run(|host| {
            let x = host.presentation().text().items()[0]
                .layout()
                .cluster_rect(3)?
                .left;
            host.begin_text_selection([x.ceil() as i32, 8])?;
            host.update_text_selection([x.ceil() as i32, 8], true)?;
            Ok(())
        })
        .unwrap();
    window
}
#[test]
fn text_selection_vertical_hwnd_keeps_column_across_short_rows_and_scrolls_endpoint() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = multiline();
    unsafe {
        assert!(key(&window, WM_KEYDOWN, VK_DOWN.0, true));
        assert_eq!(selected(&window).as_deref(), Some("W\nI"));
        assert!(key(&window, WM_KEYUP, VK_DOWN.0, true));
        assert!(key(&window, WM_KEYDOWN, VK_DOWN.0, true));
        assert_eq!(selected(&window).as_deref(), Some("W\nI\nWWW"));
        assert!(
            window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .presentation()
                .text()
                .items()[0]
                .origin()[1]
                < 0.
        );
        assert!(key(&window, WM_KEYDOWN, VK_UP.0, true));
        assert_eq!(selected(&window).as_deref(), Some("W\nI"));
        assert!(key(&window, WM_KEYDOWN, VK_UP.0, true));
        assert_eq!(selected(&window), None);
        assert!(!key(&window, WM_KEYDOWN, VK_DOWN.0, false));
        assert!(!key(&window, WM_CHAR, VK_DOWN.0, true));
        window.set_input_policy(false, false).unwrap();
        assert!(!key(&window, WM_KEYDOWN, VK_DOWN.0, true));
    }
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn text_selection_vertical_real_key_route_allows_repeat_without_double_scroll() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = multiline();
    unsafe {
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
        let _restore = Keys(original);
        let mut keys = [0; 256];
        keys[VK_SHIFT.0 as usize] = 0x80;
        SetKeyboardState(&keys).unwrap();
        for lp in [0, 1 << 30] {
            SendMessageW(
                window.hwnd,
                WM_KEYDOWN,
                Some(WPARAM(VK_DOWN.0 as usize)),
                Some(LPARAM(lp)),
            );
        }
        assert_eq!(selected(&window).as_deref(), Some("W\nI\nWWW"));
        assert_eq!(GetFocus(), window.hwnd);
        let other = fixture();
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_DOWN.0 as usize)),
            None,
        );
        assert_eq!(selected(&window).as_deref(), Some("W\nI\nWWW"));
        drop(other);
    }
    assert!(!window.has_pending_click());
    assert!(window.take_error().is_none());
}

#[test]
fn text_selection_line_edge_hwnd_extends_current_row_and_preserves_other_routes() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = multiline();
    unsafe {
        assert!(key(&window, WM_KEYDOWN, VK_END.0, true));
        assert_eq!(selected(&window).as_deref(), Some("W"));
        assert!(key(&window, WM_KEYUP, VK_END.0, true));
        assert!(key(&window, WM_KEYDOWN, VK_HOME.0, true));
        assert_eq!(selected(&window).as_deref(), Some("WWW"));
        assert!(key(&window, WM_KEYDOWN, VK_END.0, true));
        assert!(key(&window, WM_KEYDOWN, VK_DOWN.0, true));
        assert_eq!(selected(&window).as_deref(), Some("W\nI"));
        assert!(key(&window, WM_KEYDOWN, VK_HOME.0, true));
        assert_eq!(selected(&window).as_deref(), Some("W\n"));
        assert!(!key(&window, WM_KEYDOWN, VK_HOME.0, false));
        assert!(!key(&window, WM_CHAR, VK_END.0, true));
        window.set_input_policy(false, false).unwrap();
        assert!(!key(&window, WM_KEYDOWN, VK_END.0, true));
    }
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn text_selection_line_edge_real_message_does_not_advance_on_repeat_or_foreign_focus() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = multiline();
    unsafe {
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
        let _restore = Keys(original);
        let mut keys = [0; 256];
        keys[VK_SHIFT.0 as usize] = 0x80;
        SetKeyboardState(&keys).unwrap();
        for lp in [0, 1 << 30] {
            SendMessageW(
                window.hwnd,
                WM_KEYDOWN,
                Some(WPARAM(VK_END.0 as usize)),
                Some(LPARAM(lp)),
            );
        }
        assert_eq!(selected(&window).as_deref(), Some("W"));
        let other = fixture();
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_HOME.0 as usize)),
            None,
        );
        assert_eq!(selected(&window).as_deref(), Some("W"));
        drop(other);
    }
    assert!(!window.has_pending_click());
    assert!(window.take_error().is_none());
}
