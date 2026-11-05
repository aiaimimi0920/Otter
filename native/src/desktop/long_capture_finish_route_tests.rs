//! 路由验证使用隐藏系统窗口，不注册或注入用户全局热键。
use super::*;
unsafe fn window() -> crate::desktop::Window {
    crate::desktop::Window(
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("finish route test"),
            WS_OVERLAPPED,
            0,
            0,
            1,
            1,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    )
}
unsafe fn take(hwnd: HWND) -> Option<MSG> {
    let mut msg = MSG::default();
    PeekMessageW(&mut msg, Some(hwnd), MESSAGE, MESSAGE, PM_REMOVE)
        .as_bool()
        .then_some(msg)
}
#[test]
fn notifications_are_coalesced_and_bound_to_current_capture_identity() {
    unsafe {
        let window = window();
        let capture = Rc::new(Cell::new(true));
        let registration = Registration::new(window.0, capture.clone()).unwrap();
        assert!(!request(&Rc::new(Cell::new(true))));
        capture.set(false);
        assert!(!request(&capture));
        capture.set(true);
        for _ in 0..10 {
            assert!(request(&capture));
        }
        let message = take(window.0).unwrap();
        assert!(take(window.0).is_none());
        assert!(!registration.accept(window.0, WPARAM(0), message.lParam));
        assert!(!registration.accept(window.0, message.wParam, LPARAM(1)));
        capture.set(false);
        assert!(!registration.accept(window.0, message.wParam, message.lParam));
        capture.set(true);
        assert!(registration.accept(window.0, message.wParam, message.lParam));
        assert!(request(&capture));
        assert!(take(window.0).is_some());
        drop(registration);
        assert!(!request(&capture));
        assert!(GetPropW(window.0, OWNER).is_invalid());
        assert_eq!(Rc::strong_count(&capture), 1);
    }
}
#[test]
fn old_notification_and_owner_drop_cannot_finish_or_retire_a_replacement() {
    unsafe {
        let window = window();
        let capture = Rc::new(Cell::new(true));
        let old = Registration::new(window.0, capture.clone()).unwrap();
        assert!(Registration::new(window.0, capture.clone()).is_err());
        assert!(request(&capture));
        let old_message = take(window.0).unwrap();
        RemovePropW(window.0, OWNER).unwrap();
        let replacement = Registration::new(window.0, capture.clone()).unwrap();
        assert!(!old.accept(window.0, old_message.wParam, old_message.lParam));
        assert!(!replacement.accept(window.0, old_message.wParam, old_message.lParam));
        drop(old);
        assert!(request(&capture));
        let current = take(window.0).unwrap();
        assert_ne!(old_message.wParam, current.wParam);
        assert!(replacement.accept(window.0, current.wParam, current.lParam));
        drop(window);
        assert!(!request(&capture));
        drop(replacement);
        assert_eq!(Rc::strong_count(&capture), 1);
    }
}
