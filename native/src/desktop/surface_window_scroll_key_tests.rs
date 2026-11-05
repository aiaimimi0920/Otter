//! 真实 HWND 键盘滚动；父窗口门禁隔离系统 EDIT，滚动不复活旧动作。
use super::*;
use windows::Win32::UI::Controls::{EM_GETSEL, EM_SETSEL};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
#[test]
fn surface_scroll_keyboard_hwnd_background_anchor_works_without_submit_permission() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"inner","type":"scroll","layout":{"height":80,"minHeight":80},"children":[{"id":"content","type":"view","layout":{"height":300,"minHeight":300}}]},{"id":"tail","type":"view","layout":{"height":100,"minHeight":100}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 200).unwrap();
    window.set_input_policy(true, false).unwrap();
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_LBUTTONDOWN,
            None,
            Some(LPARAM((5 << 16) | 5)),
        );
        assert_eq!(GetFocus(), window.hwnd);
        assert!(window.state.keyboard.borrow().focused().is_none());
        key(&window, WM_KEYDOWN, VK_END, false);
        assert_eq!(y(&window), 0.);
        assert_eq!(
            window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .presentation()
                .geometry()
                .nodes()[2]
                .origin()[1],
            -220.
        );
        assert!(window.take_click().is_none());
    }
    assert!(window.take_error().is_none());
}
unsafe fn key(window: &SurfaceWindow, message: u32, key: VIRTUAL_KEY, repeat: bool) {
    SendMessageW(
        window.hwnd,
        message,
        Some(WPARAM(key.0 as usize)),
        Some(LPARAM(if repeat { 1 << 30 } else { 0 })),
    );
}
fn y(window: &SurfaceWindow) -> f32 {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .geometry()
        .nodes()[1]
        .origin()[1]
}
#[test]
fn surface_scroll_keyboard_hwnd_pages_repeat_gate_and_character_separation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    unsafe {
        SetFocus(Some(window.hwnd)).unwrap();
        key(&window, WM_KEYDOWN, VK_DOWN, false);
        assert_eq!(y(&window), 0.);
        window.set_input_policy(true, false).unwrap();
        key(&window, WM_KEYDOWN, VK_DOWN, false);
        key(&window, WM_KEYDOWN, VK_DOWN, true);
        assert_eq!(y(&window), -80.);
        key(&window, WM_CHAR, VK_DOWN, false);
        key(&window, WM_KEYUP, VK_DOWN, false);
        assert_eq!(y(&window), -80.);
        key(&window, WM_KEYDOWN, VK_NEXT, false);
        assert_eq!(y(&window), -180.);
        key(&window, WM_KEYDOWN, VK_HOME, false);
        assert_eq!(y(&window), 0.);
        assert!(window.take_click().is_none());
        window.set_input_policy(false, false).unwrap();
        key(&window, WM_KEYDOWN, VK_END, false);
        assert_eq!(y(&window), 0.);
    }
    assert!(window.take_error().is_none());
}
#[test]
fn surface_scroll_keyboard_hwnd_preserves_focus_but_clears_button_press() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"height":100},"children":[{"id":"button","type":"button","props":{"label":"Run"},"layout":{"height":40,"minHeight":40},"events":{"click":"run"}},{"id":"tail","type":"view","layout":{"height":300,"minHeight":300}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        SetFocus(Some(window.hwnd)).unwrap();
        key(&window, WM_KEYDOWN, VK_TAB, false);
        key(&window, WM_KEYDOWN, VK_SPACE, false);
        assert_eq!(window.state.keyboard.borrow().pressed(), Some("button"));
        key(&window, WM_KEYDOWN, VK_DOWN, false);
        assert_eq!(window.state.keyboard.borrow().focused(), Some("button"));
        assert!(window.state.keyboard.borrow().pressed().is_none());
        key(&window, WM_KEYUP, VK_SPACE, false);
        assert!(window.take_click().is_none());
        key(&window, WM_KEYDOWN, VK_HOME, false);
        key(&window, WM_KEYDOWN, VK_RETURN, false);
        assert!(window.has_pending_click());
        key(&window, WM_KEYDOWN, VK_DOWN, false);
        assert!(!window.has_pending_click());
    }
    assert!(window.take_error().is_none());
}
#[test]
fn surface_scroll_keyboard_hwnd_child_edit_retains_arrow_home_and_text() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let child = FindWindowExW(Some(window.hwnd), None, w!("EDIT"), None).unwrap();
        SetFocus(Some(child)).unwrap();
        SendMessageW(child, EM_SETSEL, Some(WPARAM(3)), Some(LPARAM(3)));
        SendMessageW(child, WM_KEYDOWN, Some(WPARAM(VK_HOME.0 as usize)), None);
        let selection = SendMessageW(child, EM_GETSEL, None, None);
        assert_eq!(selection.0, 0);
        // 即便给父窗口投递滚动键，实际焦点不在父窗口也不允许滚动。
        key(&window, WM_KEYDOWN, VK_NEXT, false);
        assert_eq!(y(&window), 0.);
        assert_eq!(GetFocus(), child);
        let mut text = [0u16; 32];
        let n = GetWindowTextW(child, &mut text);
        assert_eq!(String::from_utf16(&text[..n as usize]).unwrap(), "draft");
    }
    assert!(window.take_error().is_none());
}
