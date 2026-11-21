//! 呈现 owner 临时借出不等于画面失效；真实 resize/关闭仍撤销坐标。
use super::*;
use crate::live_input::Event;

unsafe fn wheel(window: &LiveWindow) {
    let mut point = POINT { x: 64, y: 64 };
    assert!(ClientToScreen(window.hwnd, &mut point).as_bool());
    SendMessageW(
        window.hwnd,
        WM_MOUSEWHEEL,
        Some(WPARAM(120 << 16)),
        Some(LPARAM(
            (u32::from(point.x as i16 as u16) | (u32::from(point.y as i16 as u16) << 16)) as isize,
        )),
    );
}

#[test]
fn reentrant_wheel_keeps_committed_view_and_queued_input() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        window.allow_input().unwrap();
        window.request_default_input();
        assert_eq!(window.take_input_request(), Some(true));
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.input_status(true, false, None);
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        window
            .state
            .with_host(|_| {
                assert!(window.state.host.borrow().is_none());
                wheel(&window);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            window.take_input_request(),
            None,
            "temporary Host borrow must not suspend input"
        );
        assert_eq!(
            window.pop_input(),
            Some(Event::Key {
                code: 65,
                down: true
            })
        );
        assert!(matches!(
            window.pop_input(),
            Some(Event::Wheel { delta: 120, .. })
        ));
        assert!(window.pop_input().is_none());
        assert!(window.take_error().is_none());
    }
}

#[test]
fn changed_size_during_host_borrow_rejects_old_coordinates() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        window.allow_input().unwrap();
        window.request_default_input();
        window.take_input_request();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.input_status(true, false, None);
        window
            .state
            .with_host(|_| {
                SetWindowPos(
                    window.hwnd,
                    None,
                    0,
                    0,
                    256,
                    128,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                )
                .unwrap();
                wheel(&window);
                Ok(())
            })
            .unwrap();
        assert_eq!(
            window.take_input_request(),
            None,
            "stale coordinates are rejected without retiring an idle control cycle"
        );
        assert!(window.popup_control_allowed());
        assert!(window.pop_input().is_none());
        assert!(window.take_error().is_none());
    }
}

#[test]
fn suspended_or_closed_host_invalidates_geometry_even_while_borrowed() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        for close in [false, true] {
            let source = Source::new();
            let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
            window.allow_input().unwrap();
            window.request_default_input();
            window.take_input_request();
            publish(&source);
            SendMessageW(window.hwnd, WM_PAINT, None, None);
            window.input_status(true, false, None);
            window
                .state
                .with_host(|host| {
                    if close {
                        host.close();
                    } else {
                        host.suspend();
                    }
                    wheel(&window);
                    Ok(())
                })
                .unwrap();
            assert_eq!(
                window.take_input_request(),
                None,
                "unavailable geometry alone does not create a cancel request"
            );
            assert!(window.popup_control_allowed());
            assert!(window.pop_input().is_none());
        }
    }
}
