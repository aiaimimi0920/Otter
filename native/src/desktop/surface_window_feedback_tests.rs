//! 真实 HWND 悬浮/按下/离开状态与按需重绘合同。
use super::*;

#[test]
fn surface_button_feedback_tracks_hover_press_release_leave_and_disable() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(Scene::parse(br#"{"id":"button","type":"button","props":{"label":"Run"},"layout":{"width":120,"height":60}}"#).unwrap(), 240, 160).unwrap();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        send(&window, WM_MOUSEMOVE, 10, 10);
        assert_eq!(window.state.pointer.borrow().hovered(), Some("button"));
        let _ = ValidateRect(Some(window.hwnd), None);
        send(&window, WM_MOUSEMOVE, 11, 11);
        assert!(!GetUpdateRect(window.hwnd, None, false).as_bool());
        send(&window, WM_LBUTTONDOWN, 10, 10);
        assert_eq!(window.state.pointer.borrow().pressed_node(), Some("button"));
        send(&window, WM_PAINT, 0, 0);
        assert!(window.take_error().is_none());
        send(&window, WM_LBUTTONUP, 10, 10);
        assert!(window.state.pointer.borrow().pressed_node().is_none());
        assert_eq!(window.state.pointer.borrow().hovered(), Some("button"));
        send(&window, WM_MOUSELEAVE, 0, 0);
        assert!(window.state.pointer.borrow().hovered().is_none());
        window.take_click();
        send(&window, WM_LBUTTONDOWN, 10, 10);
        ReleaseCapture().unwrap();
        assert!(window.state.pointer.borrow().pressed_node().is_none());
        assert!(window.state.pointer.borrow().hovered().is_none());
        send(&window, WM_MOUSEMOVE, 10, 10);
        window.set_pointer_enabled(false).unwrap();
        send(&window, WM_MOUSEMOVE, 10, 10);
        assert!(window.state.pointer.borrow().hovered().is_none());
    }
}
