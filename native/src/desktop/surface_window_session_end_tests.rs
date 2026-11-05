//! 系统准入封住父窗口和既有EDIT subclass；真实文本/通知、取消恢复与销毁可独立观察。
use super::*;
use crate::desktop::session_end::{with_owner, with_query, Control};
#[test]
fn approved_session_blocks_actual_child_edit_and_cancel_restores_delivery() {
    let _apartment = crate::desktop::Apartment::new().unwrap();
    let control = Rc::new(Control::default());
    with_owner(&control, || unsafe {
        let window = mount();
        let edit = child(&window);
        assert!(window.session_end_idle());
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
        assert!(!window.session_end_idle());
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
        assert_eq!(
            SendMessageW(window.hwnd, WM_QUERYENDSESSION, None, None).0,
            0
        );
        assert_eq!(
            with_query(
                &control,
                || true,
                || SendMessageW(window.hwnd, WM_QUERYENDSESSION, None, None).0
            ),
            1
        );
        type_text(edit, "must not be accepted");
        SendMessageW(window.hwnd, WM_CLOSE, None, None);
        assert_eq!(text(edit), "source");
        assert!(!window.has_pending_edit());
        assert!(window.is_alive());
        SendMessageW(window.hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
        type_text(edit, "accepted after cancellation");
        assert_eq!(text(edit), "accepted after cancellation");
        assert!(window.has_pending_edit());
        assert!(!window.session_end_idle());
        SendMessageW(window.hwnd, WM_ENDSESSION, Some(WPARAM(1)), None);
        window.close().unwrap();
        assert!(!IsWindow(Some(edit)).as_bool());
        assert!(window.state.host.borrow().is_none());
        assert!(window.state.uia.borrow().is_none());
        let mut message = MSG::default();
        assert!(PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool());
    });
}
