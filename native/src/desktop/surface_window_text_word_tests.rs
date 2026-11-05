//! 真实 HWND 双击与词级拖动；不发送系统输入或写剪贴板。
use super::*;
#[test]
fn word_double_click_cross_node_drag_retains_capture_and_copy_gate() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let scene = Scene::parse(br#"{"id":"root","type":"row","events":{"click":"ancestor"},"children":[{"id":"a","type":"text","props":{"text":"Alpha","selectable":true},"layout":{"width":100,"height":32}},{"id":"b","type":"text","props":{"text":"Bravo","selectable":true},"layout":{"width":100,"height":32}}]}"#).unwrap();
    window.replace(scene).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDBLCLK, 1, 8);
        assert_eq!(selected(&window).as_deref(), Some("Alpha"));
        send(&window, WM_MOUSEMOVE, 110, 8);
        assert_eq!(GetCapture(), window.hwnd);
        assert!(selected(&window).is_none());
        send(&window, WM_MOUSEMOVE, 1, 8);
        assert_eq!(selected(&window).as_deref(), Some("Alpha"));
        send(&window, WM_LBUTTONUP, 110, 8);
        assert_ne!(GetCapture(), window.hwnd);
    }
    assert!(selected(&window).is_none());
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn word_double_click_nonselectable_content_retains_click_contract() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"text","props":{"text":"ordinary text"},"layout":{"width":200,"height":40},"events":{"click":"ordinary"}}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 120).unwrap();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDBLCLK, 10, 8);
        send(&window, WM_LBUTTONUP, 10, 8);
    }
    assert!(window.take_click().is_some());
    assert!(selected(&window).is_none());
    assert!(window.take_error().is_none());
}
#[test]
fn word_double_click_reuses_capture_and_finishes_without_ancestor_action() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    unsafe {
        assert_ne!(GetClassLongW(window.hwnd, GCL_STYLE) & CS_DBLCLKS.0, 0);
        send(&window, WM_LBUTTONDOWN, 10, 8);
        send(&window, WM_LBUTTONUP, 10, 8);
        assert!(window.take_click().is_some()); // existing first-click delivery is retained
        send(&window, WM_LBUTTONDBLCLK, 10, 8);
        assert_eq!(GetCapture(), window.hwnd);
        assert_eq!(selected(&window).as_deref(), Some("Native "));
        send(&window, WM_LBUTTONUP, 10, 8);
        assert_ne!(GetCapture(), window.hwnd);
    }
    assert_eq!(selected(&window).as_deref(), Some("Native "));
    assert!(!window.has_pending_click());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn word_double_click_drag_extends_by_words_and_can_reverse() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    unsafe {
        send(&window, WM_LBUTTONDBLCLK, 80, 8);
        assert_eq!(selected(&window).as_deref(), Some("selection"));
        send(&window, WM_MOUSEMOVE, 1, 8);
        assert_eq!(selected(&window).as_deref(), Some("Native selection"));
        send(&window, WM_MOUSEMOVE, 80, 8);
        assert_eq!(selected(&window).as_deref(), Some("selection"));
        send(&window, WM_LBUTTONUP, 1, 8);
    }
    assert_eq!(selected(&window).as_deref(), Some("Native selection"));
    assert!(!window.has_pending_click());
    assert!(window.take_error().is_none());
}
#[test]
fn word_double_click_cancel_revoke_replace_and_close_do_not_revive() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for mode in 0..4 {
        let window = window();
        unsafe {
            send(&window, WM_LBUTTONDBLCLK, 10, 8);
        }
        assert!(selected(&window).is_some());
        match mode {
            0 => unsafe {
                send(&window, WM_CANCELMODE, 0, 0);
            },
            1 => window.set_input_policy(false, false).unwrap(),
            2 => window.replace(scene()).unwrap(),
            _ => {
                window.close().unwrap();
                continue;
            }
        }
        unsafe {
            send(&window, WM_MOUSEMOVE, 200, 8);
            send(&window, WM_LBUTTONUP, 200, 8);
        }
        assert!(selected(&window).is_none());
        assert!(!window.has_pending_click());
        assert!(window.take_error().is_none());
        unsafe {
            assert_ne!(GetCapture(), window.hwnd);
        }
    }
}
