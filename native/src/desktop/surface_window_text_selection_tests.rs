//! 真实自有 HWND 消息验证拖选与 click/捕获边界；不注入外部桌面，不读写系统剪贴板。
use super::*;
#[path = "surface_text_mouse_probe_tests.rs"]
mod system_mouse;
#[path = "surface_window_text_word_tests.rs"]
mod word;
#[test]
fn cross_node_hwnd_drag_keeps_visual_gesture_without_text_copy_or_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let row = Scene::parse(br#"{"id":"root","type":"row","events":{"click":"ancestor"},"children":[{"id":"a","type":"text","props":{"text":"Alpha","selectable":true},"layout":{"width":60,"height":32}},{"id":"b","type":"text","props":{"text":"Bravo","selectable":true},"layout":{"width":60,"height":32}},{"id":"c","type":"text","props":{"text":"Charlie","selectable":true},"layout":{"width":60,"height":32}}]}"#).unwrap();
    window.replace(row).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 1, 8);
        send(&window, WM_MOUSEMOVE, 150, 8);
        assert_eq!(GetCapture(), window.hwnd);
        assert!(selected(&window).is_none());
        // 返回 anchor owner 后恢复同节点文字复制，证明跨节点不是清掉整个选择。
        send(&window, WM_MOUSEMOVE, 30, 8);
        assert!(selected(&window).is_some());
        send(&window, WM_LBUTTONUP, 150, 8);
        assert_ne!(GetCapture(), window.hwnd);
    }
    assert!(selected(&window).is_none());
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
fn scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"stack","layout":{"width":220,"height":100},"events":{"click":"copy-block"},"children":[{"id":"text","type":"text","props":{"text":"Native selection","selectable":true},"layout":{"width":200,"height":40}}]}"#).unwrap()
}
fn window() -> SurfaceWindow {
    let window = SurfaceWindow::new(scene(), 240, 120).unwrap();
    window.set_pointer_enabled(true).unwrap();
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
unsafe fn send(window: &SurfaceWindow, message: u32, x: i16, y: i16) {
    SendMessageW(
        window.hwnd,
        message,
        Some(WPARAM(0)),
        Some(LPARAM(((y as u16 as u32) << 16 | x as u16 as u32) as isize)),
    );
}
#[test]
fn text_selection_hwnd_drag_retains_range_and_suppresses_ancestor_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 1, 8);
        assert_eq!(GetCapture(), window.hwnd);
        send(&window, WM_MOUSEMOVE, 210, 8);
        assert_eq!(selected(&window).as_deref(), Some("Native selection"));
        send(&window, WM_LBUTTONUP, 210, 8);
        assert_ne!(GetCapture(), window.hwnd);
    }
    assert_eq!(selected(&window).as_deref(), Some("Native selection"));
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn text_selection_hwnd_collapsed_click_preserves_original_click_contract() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 1, 8);
        send(&window, WM_LBUTTONUP, 1, 8);
    }
    assert_eq!(selected(&window), None);
    assert!(window.has_pending_click());
    assert!(window.take_error().is_none());
}
#[test]
fn text_selection_hwnd_cancel_replace_and_revoke_reject_late_drag() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for mode in 0..3 {
        let window = window();
        unsafe {
            send(&window, WM_LBUTTONDOWN, 1, 8);
            send(&window, WM_MOUSEMOVE, 100, 8);
        }
        assert!(selected(&window).is_some());
        match mode {
            0 => unsafe {
                send(&window, WM_CANCELMODE, 0, 0);
            },
            1 => window.replace(scene()).unwrap(),
            _ => window.set_input_policy(false, false).unwrap(),
        }
        unsafe {
            send(&window, WM_MOUSEMOVE, 210, 8);
            send(&window, WM_LBUTTONUP, 210, 8);
        }
        assert_eq!(selected(&window), None);
        assert!(!window.has_pending_click());
        assert!(!window.has_pending_edit());
        unsafe {
            assert_ne!(GetCapture(), window.hwnd);
        }
        assert!(window.take_error().is_none());
    }
}
#[test]
fn text_selection_hwnd_reentrant_cancel_is_applied_before_host_returns() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 1, 8);
        send(&window, WM_MOUSEMOVE, 100, 8);
    }
    assert!(selected(&window).is_some());
    window
        .state
        .run(|host| {
            assert!(host.presentation().text_selection_dragging());
            unsafe {
                send(&window, WM_CANCELMODE, 0, 0);
            }
            Ok(())
        })
        .unwrap();
    assert_eq!(selected(&window), None);
    assert_eq!(window.state.pointer.borrow().selection_cancel, 0);
    unsafe {
        send(&window, WM_LBUTTONUP, 210, 8);
        assert_ne!(GetCapture(), window.hwnd);
    }
    assert!(!window.has_pending_click());
    assert!(window.take_error().is_none());
}
#[test]
fn text_selection_hwnd_completed_range_survives_resize_but_not_revoke() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 1, 8);
        send(&window, WM_LBUTTONUP, 210, 8);
        SetWindowPos(
            window.hwnd,
            None,
            0,
            0,
            280,
            160,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        )
        .unwrap();
    }
    window
        .state
        .run(|host| {
            host.hit_test([1, 8])?;
            Ok(())
        })
        .unwrap();
    assert_eq!(selected(&window).as_deref(), Some("Native selection"));
    window.set_submission_enabled(false).unwrap();
    assert!(selected(&window).is_some());
    window.set_input_policy(false, false).unwrap();
    assert_eq!(selected(&window), None);
    assert!(window.take_error().is_none());
}
