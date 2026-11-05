//! 复用隐藏Live/WARP源验证共享系统准入、业务冻结及原销毁；不启动采集或注入用户输入。
use super::*;
use crate::desktop::session_end::{with_owner, with_query, Control};
#[test]
fn hidden_live_rejects_unscoped_query_and_freezes_until_cancel_or_end() {
    let _apartment = Apartment::new().unwrap();
    let control = std::rc::Rc::new(Control::default());
    with_owner(&control, || unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        window.allow_input().unwrap();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.input_status(true, false, None);
        assert_eq!(
            SendMessageW(window.hwnd, WM_QUERYENDSESSION, None, None).0,
            0
        );
        assert!(window.state.session_end_reason.get());
        assert_eq!(
            with_query(
                &control,
                || true,
                || SendMessageW(window.hwnd, WM_QUERYENDSESSION, None, None).0
            ),
            1
        );
        assert!(!window.state.session_end_reason.get());
        for command in [0x1fb0, 0x1fa0, 0x1fc0, 0x1fd0, 0x1fe0] {
            SendMessageW(window.hwnd, WM_SYSCOMMAND, Some(WPARAM(command)), None);
        }
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert!(window.is_alive());
        assert!(window.exit_idle());
        assert!(window.pop_input().is_none());
        SendMessageW(window.hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
        assert!(!control.paused());
        SendMessageW(window.hwnd, WM_SYSCOMMAND, Some(WPARAM(0x1fb0)), None);
        assert!(!window.exit_idle());
        assert!(window.take_snapshot_request());
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert!(window.pop_input().is_some());
        assert_eq!(
            with_query(
                &control,
                || true,
                || SendMessageW(window.hwnd, WM_QUERYENDSESSION, None, None).0
            ),
            1
        );
        SendMessageW(window.hwnd, WM_ENDSESSION, Some(WPARAM(1)), None);
        let hwnd = window.hwnd;
        drop(window);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert!(control.ended());
        let mut message = MSG::default();
        assert!(PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool());
        assert_eq!(message.wParam.0, 0);
    });
}
