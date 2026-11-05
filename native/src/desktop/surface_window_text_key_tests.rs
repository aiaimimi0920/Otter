//! 正式 HWND/Host 键盘路由；参数或可恢复的线程键盘状态模拟修饰键，不注入桌面输入。
use super::*;
#[path = "surface_window_text_vertical_tests.rs"]
mod vertical;
struct Keys([u8; 256]);
impl Drop for Keys {
    fn drop(&mut self) {
        unsafe {
            let _ = SetKeyboardState(&self.0);
        }
    }
}
fn fixture() -> SurfaceWindow {
    let scene = Scene::parse(br#"{"id":"text","type":"text","props":{"text":"Native selection","selectable":true},"layout":{"width":200,"height":40}}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 120).unwrap();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        keyboard::focus_node(window.hwnd, &window.state, "text");
    }
    window
        .state
        .run(|host| {
            host.begin_text_selection([1, 8])?;
            host.update_text_selection([1, 8], true)?;
            Ok(())
        })
        .unwrap();
    window
}
fn selected(window: &SurfaceWindow) -> Option<String> {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .selected_text()
        .unwrap()
}
#[test]
fn cross_node_keyboard_route_keeps_navigation_owner_and_returns_to_text_copy() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"row","children":[{"id":"a","type":"text","props":{"text":"Alpha","selectable":true},"layout":{"width":100,"height":32}},{"id":"b","type":"text","props":{"text":"Bravo","selectable":true},"layout":{"width":100,"height":32}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 120).unwrap();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        keyboard::focus_node(window.hwnd, &window.state, "a");
    }
    window
        .state
        .run(|host| {
            host.begin_text_selection([1, 8])?;
            host.update_text_selection([101, 8], true)?;
            Ok(())
        })
        .unwrap();
    assert!(selected(&window).is_none());
    unsafe {
        assert!(key(&window, WM_KEYDOWN, VK_UP.0, true));
        assert!(key(&window, WM_KEYUP, VK_UP.0, true));
        assert_eq!(selected(&window).as_deref(), Some("Alpha"));
        assert!(key(&window, WM_KEYDOWN, VK_DOWN.0, true));
        assert!(selected(&window).is_none());
        assert!(!key(&window, WM_KEYDOWN, VK_RIGHT.0, false));
        assert!(key(&window, WM_KEYDOWN, VK_LEFT.0, true));
        assert!(key(&window, WM_KEYUP, VK_LEFT.0, true));
    }
    assert_eq!(selected(&window).as_deref(), Some("Alpha"));
    unsafe {
        assert!(key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
    }
    assert!(selected(&window).is_none());
    assert_eq!(window.state.keyboard.borrow().anchor(), Some("a"));
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
unsafe fn visible_region(hwnd: HWND) -> bool {
    let region = CreateRectRgn(0, 0, 0, 0);
    assert!(!region.is_invalid());
    let kind = GetWindowRgn(hwnd, region);
    let _ = DeleteObject(region.into());
    assert_ne!(kind, RGN_ERROR);
    kind != NULLREGION
}
unsafe fn key(window: &SurfaceWindow, message: u32, key: u16, extend: bool) -> bool {
    route(window.hwnd, &window.state, message, key, extend, false)
}
#[test]
fn text_selection_keyboard_routes_repeat_release_and_collapse_without_remote_events() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = fixture();
    unsafe {
        assert!(key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
        assert_eq!(selected(&window).as_deref(), Some("N"));
        assert!(key(&window, WM_KEYUP, VK_RIGHT.0, true));
        assert_eq!(selected(&window).as_deref(), Some("N"));
        assert!(key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
        assert_eq!(selected(&window).as_deref(), Some("Na"));
        assert!(!key(&window, WM_CHAR, VK_RIGHT.0, true));
        assert!(key(&window, WM_KEYDOWN, VK_LEFT.0, false));
        assert_eq!(selected(&window), None);
        // 折叠后普通方向键回到原滚动路径，Shift 可继续从保留锚点扩选。
        assert!(!key(&window, WM_KEYDOWN, VK_LEFT.0, false));
        assert!(key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
    }
    assert_eq!(selected(&window).as_deref(), Some("N"));
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn text_selection_keyboard_respects_focus_modifiers_and_independent_local_policy() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = fixture();
    unsafe {
        assert!(!route(
            window.hwnd,
            &window.state,
            WM_KEYDOWN,
            VK_RIGHT.0,
            true,
            true
        ));
        window.set_submission_enabled(false).unwrap();
        assert!(key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
        let other = fixture();
        assert!(!key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
        drop(other);
        keyboard::focus_node(window.hwnd, &window.state, "different");
        assert!(!key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
        keyboard::focus_node(window.hwnd, &window.state, "text");
        assert!(key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
        window.set_input_policy(false, false).unwrap();
        assert!(!key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
    }
    assert_eq!(selected(&window), None);
    assert!(window.take_error().is_none());
}
#[test]
fn text_selection_keyboard_never_extends_active_drag_or_replaced_scene() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = fixture();
    window
        .state
        .run(|host| {
            host.begin_text_selection([1, 8])?;
            Ok(())
        })
        .unwrap();
    unsafe {
        assert!(!key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
    }
    window
        .replace(
            Scene::parse(
                br#"{"id":"text","type":"text","props":{"text":"replacement","selectable":true}}"#,
            )
            .unwrap(),
        )
        .unwrap();
    unsafe {
        assert!(!key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
    }
    assert_eq!(selected(&window), None);
    assert!(!window.has_pending_click());
}

#[test]
fn text_selection_keyboard_real_hwnd_route_precedes_scroll_and_keeps_shift_tab_available() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = fixture();
    unsafe {
        let mut original = [0; 256];
        GetKeyboardState(&mut original).unwrap();
        let _restore = Keys(original);
        let mut keys = [0; 256];
        keys[VK_SHIFT.0 as usize] = 0x80;
        SetKeyboardState(&keys).unwrap();
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_RIGHT.0 as usize)),
            Some(LPARAM(0)),
        );
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_RIGHT.0 as usize)),
            Some(LPARAM(1 << 30)),
        );
        SendMessageW(
            window.hwnd,
            WM_KEYUP,
            Some(WPARAM(VK_RIGHT.0 as usize)),
            Some(LPARAM(0)),
        );
        assert_eq!(selected(&window).as_deref(), Some("Na"));
        // 不截获 Shift+Tab；无其他可聚焦控件时锚点被现有导航撤销。
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_TAB.0 as usize)),
            Some(LPARAM(0)),
        );
        assert!(window.state.keyboard.borrow().anchor().is_none());
        SendMessageW(
            window.hwnd,
            WM_KEYDOWN,
            Some(WPARAM(VK_RIGHT.0 as usize)),
            Some(LPARAM(0)),
        );
        assert_eq!(selected(&window).as_deref(), Some("Na"));
    }
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn text_selection_reveal_hwnd_synchronizes_edit_visibility_and_restores_without_events() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":200,"height":60},"children":[{"id":"body","type":"stack","layout":{"position":"relative","height":300,"minHeight":300},"children":[{"id":"text","type":"text","props":{"text":"A\nB\nC\nD\nE\nF","selectable":true},"style":{"fontSize":20,"whiteSpace":"pre"},"layout":{"position":"absolute","left":0,"top":0,"width":60,"height":300}},{"id":"field","type":"input","props":{"value":"draft"},"layout":{"position":"absolute","left":90,"top":0,"width":80,"height":24}}]}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, false).unwrap();
    unsafe {
        keyboard::focus_node(window.hwnd, &window.state, "text");
        let child = FindWindowExW(Some(window.hwnd), None, w!("EDIT"), None).unwrap();
        assert!(visible_region(child));
        window
            .state
            .run(|host| {
                host.begin_text_selection([1, 8])?;
                host.update_text_selection([1, 8], true)?;
                Ok(())
            })
            .unwrap();
        for _ in 0..11 {
            assert!(key(&window, WM_KEYDOWN, VK_RIGHT.0, true));
        }
        assert_eq!(selected(&window).as_deref(), Some("A\nB\nC\nD\nE\nF"));
        assert!(!visible_region(child));
        assert!(key(&window, WM_KEYDOWN, VK_LEFT.0, false));
        assert!(visible_region(child));
        assert_eq!(GetFocus(), window.hwnd);
        let mut text = [0u16; 32];
        let len = GetWindowTextW(child, &mut text);
        assert_eq!(String::from_utf16(&text[..len as usize]).unwrap(), "draft");
    }
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
