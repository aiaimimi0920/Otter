use super::*;
use crate::{
    desktop::{Apartment, Window},
    live_frames::Source,
};
use windows::core::w;
unsafe fn window() -> Window {
    Window(
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!(""),
            WS_POPUP,
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
unsafe fn next(hwnd: HWND) -> Option<MSG> {
    let mut message = MSG::default();
    PeekMessageW(&mut message, Some(hwnd), MESSAGE, MESSAGE, PM_REMOVE)
        .as_bool()
        .then_some(message)
}
#[test]
fn hidden_window_receives_one_notification_for_many_frames_and_close() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = window();
        let source = Source::new();
        let mut signal = WindowSignal::bind(window.0, source.subscribe().unwrap()).unwrap();
        for value in 1..=100 {
            source.publish(value).unwrap();
        }
        let message = next(window.0).unwrap();
        assert!(next(window.0).is_none());
        assert!(signal
            .receive(window.0, MESSAGE, WPARAM(0), LPARAM(0))
            .is_none());
        let delivery = signal
            .receive(window.0, message.message, message.wParam, message.lParam)
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(delivery.frame.resource, 100);
        assert_eq!(delivery.skipped, 99);
        source.close();
        let message = next(window.0).unwrap();
        assert!(matches!(
            signal.receive(window.0, message.message, message.wParam, message.lParam),
            Some(Err(crate::live_frames::Error::Closed))
        ));
        assert!(next(window.0).is_none());
    }
}
#[test]
fn new_connection_rejects_old_queued_generation_and_invalid_window() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let window = window();
        let old = Source::new();
        let first = WindowSignal::bind(window.0, old.subscribe().unwrap()).unwrap();
        old.publish(1).unwrap();
        let stale = next(window.0).unwrap();
        drop(first);
        assert!(!old.has_consumers());
        let source = Source::new();
        source.publish(2).unwrap_err();
        let mut signal = WindowSignal::bind(window.0, source.subscribe().unwrap()).unwrap();
        assert!(signal
            .receive(window.0, stale.message, stale.wParam, stale.lParam)
            .is_none());
        source.publish(3).unwrap();
        let current = next(window.0).unwrap();
        assert_eq!(
            signal
                .receive(window.0, current.message, current.wParam, current.lParam)
                .unwrap()
                .unwrap()
                .unwrap()
                .frame
                .resource,
            3
        );
        DestroyWindow(window.0).unwrap();
        source.publish(4).unwrap();
        assert_eq!(signal.notification_failures(), 1);
        let source = Source::<u8>::new();
        assert!(WindowSignal::bind(HWND::default(), source.subscribe().unwrap()).is_err());
        assert!(!source.has_consumers());
    }
}
