//! In-memory Viewer/Permit and hidden Relay; shared admission must not receive default approval through DefWindowProc, while terminal state still releases Host.
use super::*;
use crate::desktop::session_end::{with_owner, with_query, Control};
#[test]
fn hidden_relay_uses_shared_query_and_preserves_native_destroy_cleanup() {
    let _apartment = Apartment::new().unwrap();
    let control = std::rc::Rc::new(Control::default());
    with_owner(&control, || unsafe {
        let (media, permit) = media();
        let window = RelayWindow::new(media.clone(), permit.clone(), 128, 96).unwrap();
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
        SendMessageW(window.hwnd, WM_CLOSE, None, None);
        assert!(window.is_alive());
        SendMessageW(window.hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
        assert!(!control.paused());
        SendMessageW(window.hwnd, WM_CLOSE, None, None);
        assert!(!window.is_alive());
        assert!(window.state.host.borrow().is_none());
        assert!(media.validate(&permit).is_ok());
    });
}
