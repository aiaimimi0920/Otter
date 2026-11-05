use super::*;
use windows::core::w;
unsafe fn fixture(token: usize) -> (crate::desktop::Window, Hook) {
    let window = crate::desktop::Window(
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("recording hook fixture"),
            WS_OVERLAPPED,
            -32000,
            -32000,
            1,
            1,
            None,
            None,
            None,
            None,
        )
        .unwrap(),
    );
    // 合成接缝测试：不安装系统 hook，不注入任何用户输入。
    SetPropW(window.0, PROPERTY, Some(HANDLE(token as *mut _))).unwrap();
    WINDOW.with(|value| value.set(window.0));
    ROOT.with(|value| value.set(99));
    GENERATION.with(|value| value.set(token));
    PENDING.with(|value| value.set(None));
    QUEUED.with(|value| value.set(false));
    let hook = Hook {
        hwnd: window.0,
        token,
        hook: HHOOK::default(),
        _thread: PhantomData,
    };
    (window, hook)
}
#[test]
fn thousands_of_notifications_are_one_latest_hint_and_old_generation_is_rejected() {
    unsafe {
        let (window, hook) = fixture(7);
        for index in 0..10_000 {
            notify(
                WM_MOUSEWHEEL,
                POINT {
                    x: index,
                    y: -index,
                },
                -1,
                99,
                index as u64,
            );
        }
        let mut message = MSG::default();
        assert!(PeekMessageW(&mut message, Some(window.0), MESSAGE, MESSAGE, PM_REMOVE).as_bool());
        assert_eq!(message.wParam.0, 7);
        assert!(!PeekMessageW(&mut message, Some(window.0), MESSAGE, MESSAGE, PM_REMOVE).as_bool());
        assert!(hook.take(WPARAM(6)).is_none());
        let wheel = hook.take(WPARAM(7)).unwrap();
        assert_eq!(
            (wheel.point.x, wheel.point.y, wheel.time_ms),
            (9999, -9999, 9999)
        );
        assert!(hook.take(WPARAM(7)).is_none());
        notify(WM_MOUSEHWHEEL, POINT { x: -12, y: 24 }, 1, 99, 20);
        let wheel = hook.pending().unwrap();
        assert_eq!((wheel.point.x, wheel.point.y), (-12, 24));
        drop(hook);
        assert!(WINDOW.with(Cell::get).is_invalid());
        assert!(PENDING.with(Cell::get).is_none());
        assert!(GetPropW(window.0, PROPERTY).is_invalid());
    }
}
#[test]
fn foreign_target_injected_events_and_revoked_property_cannot_publish() {
    unsafe {
        let (window, hook) = fixture(11);
        notify(WM_MOUSEWHEEL, POINT::default(), -120, 100, 1);
        notify(WM_MOUSEWHEEL, POINT::default(), 0, 99, 1);
        assert!(PENDING.with(Cell::get).is_none());
        let event = MSLLHOOKSTRUCT {
            flags: 1,
            mouseData: (120u32) << 16,
            ..Default::default()
        };
        callback(
            HC_ACTION as i32,
            WPARAM(WM_MOUSEWHEEL as usize),
            LPARAM(&event as *const _ as isize),
        );
        assert!(PENDING.with(Cell::get).is_none());
        notify(WM_MOUSEWHEEL, POINT::default(), -120, 99, 2);
        RemovePropW(window.0, PROPERTY).unwrap();
        assert!(hook.take(WPARAM(11)).is_none());
        drop(hook);
        assert!(WINDOW.with(Cell::get).is_invalid());
    }
}
