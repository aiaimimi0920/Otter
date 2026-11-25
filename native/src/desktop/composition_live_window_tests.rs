use super::*;
#[path = "composition_live_input_reentrant_tests.rs"]
mod input_reentrant;
#[path = "composition_live_pin_tests.rs"]
mod pin_tests;
#[path = "composition_live_session_end_tests.rs"]
mod session_end;
use crate::{desktop::Apartment, live_frames::Source};
use std::sync::Arc;
use windows::{core::Interface, Win32::Graphics::Direct3D11::ID3D11Multithread};

#[test]
fn suspended_live_accepts_pointer_input_without_foreground_and_keeps_explicit_stop() {
    use crate::live_input::Event;
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
        SendMessageW(window.hwnd, WM_KILLFOCUS, None, None);
        assert_eq!(window.take_input_request(), Some(false));
        assert_ne!(GetForegroundWindow(), window.hwnd);
        let mut screen = POINT { x: 64, y: 64 };
        assert!(ClientToScreen(window.hwnd, &mut screen).as_bool());
        let wheel_point = LPARAM(
            (u32::from(screen.x as i16 as u16) | (u32::from(screen.y as i16 as u16) << 16))
                as isize,
        );
        SendMessageW(
            window.hwnd,
            WM_MOUSEWHEEL,
            Some(WPARAM(120 << 16)),
            Some(wheel_point),
        );
        assert_eq!(
            window.take_input_request(),
            Some(true),
            "inactive scrolling must resume source input"
        );
        assert!(matches!(
            window.pop_input(),
            Some(Event::Wheel { delta: 120, .. })
        ));
        // Pending mouse is retained, while keyboard still awaits the worker's acknowledgement.
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert!(window.pop_input().is_none());
        window.input_status(true, false, None);
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert_eq!(
            window.pop_input(),
            Some(Event::Key {
                code: 65,
                down: true
            })
        );
        input::cancel(window.hwnd, &window.state);
        assert_eq!(window.take_input_request(), Some(false));
        SendMessageW(
            window.hwnd,
            WM_MOUSEMOVE,
            None,
            Some(LPARAM(64 | (64 << 16))),
        );
        SendMessageW(
            window.hwnd,
            WM_MOUSEWHEEL,
            Some(WPARAM(120 << 16)),
            Some(wheel_point),
        );
        assert_eq!(window.take_input_request(), None);
        assert!(window.pop_input().is_none());
    }
}

#[test]
fn default_input_survives_pointer_activity_before_the_first_presented_frame() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        window.allow_input().unwrap();
        window.request_default_input();
        SendMessageW(
            window.hwnd,
            WM_MOUSEMOVE,
            None,
            Some(LPARAM(32 | (32 << 16))),
        );
        assert_eq!(window.take_input_request(), Some(true));
        assert!(window.pop_input().is_none());
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.input_status(true, false, None);
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert_eq!(
            window.pop_input(),
            Some(crate::live_input::Event::Key {
                code: 65,
                down: true
            })
        );
    }
}

#[test]
fn pending_status_preserves_first_mouse_capture_until_source_acknowledges() {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        window.allow_input().unwrap();
        window.show().unwrap();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.request_default_input();
        assert_eq!(window.take_input_request(), Some(true));
        let point = LPARAM(64 | (64 << 16));
        SendMessageW(window.hwnd, WM_LBUTTONDOWN, None, Some(point));
        assert_eq!(GetCapture(), window.hwnd);
        window.input_status(false, true, None);
        assert_eq!(
            GetCapture(),
            window.hwnd,
            "pending acknowledgement must not cancel the gesture"
        );
        window.input_status(true, false, None);
        SendMessageW(window.hwnd, WM_LBUTTONUP, None, Some(point));
        assert!(matches!(
            window.pop_input(),
            Some(crate::live_input::Event::Button { down: true, .. })
        ));
        assert!(matches!(
            window.pop_input(),
            Some(crate::live_input::Event::Button { down: false, .. })
        ));
        // Click-driven focus may pause/resume during asynchronous admission; recovery requests are idempotent and must not cancel gestures.
        assert_ne!(window.take_input_request(), Some(false));
        assert_ne!(GetCapture(), window.hwnd);
    }
}

pub(super) unsafe fn publish(source: &Source<Texture>) {
    let device = super::super::super::create_device(
        windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_WARP,
    )
    .unwrap();
    let protection: ID3D11Multithread = device.GetImmediateContext().unwrap().cast().unwrap();
    let _ = protection.SetMultithreadProtected(true);
    let frame = super::super::frame_fixture::frame(&device, [0, 255, 0, 255]);
    source
        .publish(Arc::try_unwrap(frame).ok().unwrap().resource)
        .unwrap();
}

#[test]
fn input_menu_requires_window_capability_ack_and_focus_loss_clears_queue() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        SendMessageW(
            window.hwnd,
            WM_SYSCOMMAND,
            Some(WPARAM(input::TOGGLE)),
            None,
        );
        assert_eq!(window.take_input_request(), None);
        window.allow_input().unwrap();
        SendMessageW(
            window.hwnd,
            WM_SYSCOMMAND,
            Some(WPARAM(input::TOGGLE)),
            None,
        );
        assert_eq!(window.take_input_request(), Some(true));
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        window.input_status(false, true, None);
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert!(window.pop_input().is_none());
        window.input_status(true, false, None);
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(65)), None);
        assert_eq!(
            window.pop_input(),
            Some(crate::live_input::Event::Key {
                code: 65,
                down: true
            })
        );
        SendMessageW(window.hwnd, WM_KEYDOWN, Some(WPARAM(66)), None);
        SendMessageW(window.hwnd, WM_KILLFOCUS, None, None);
        assert_eq!(window.take_input_request(), Some(false));
        assert!(window.pop_input().is_none());
        assert_eq!(
            GetMenuState(
                GetSystemMenu(window.hwnd, false),
                input::TOGGLE as u32,
                MF_BYCOMMAND
            ) & MF_CHECKED.0,
            0
        );
        assert!(window.is_alive());
    }
}

#[test]
#[ignore = "shows owned Live fixture and uses mouse capture, without synthesizing physical input"]
fn input_mouse_letterbox_capture_release_and_capture_loss_are_safe() {
    use crate::live_input::{Button, Event};
    use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 160, 160).unwrap();
        window.allow_input().unwrap();
        window.show().unwrap();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        let _ = window.take_input_request();
        window.input_status(true, false, None);
        let mut client = RECT::default();
        GetClientRect(window.hwnd, &mut client).unwrap();
        let view = window
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .presented_viewport([client.right as u32, client.bottom as u32])
            .unwrap()
            .1;
        let [left, top, right, bottom] = view.destination();
        let center = [((left + right) * 0.5) as i32, ((top + bottom) * 0.5) as i32];
        let lp = |x: i32, y: i32| {
            LPARAM((u32::from(x as i16 as u16) | (u32::from(y as i16 as u16) << 16)) as isize)
        };
        SendMessageW(window.hwnd, WM_LBUTTONDOWN, None, Some(lp(-1, -1)));
        assert!(window.pop_input().is_none());
        SendMessageW(
            window.hwnd,
            WM_LBUTTONDOWN,
            None,
            Some(lp(center[0], center[1])),
        );
        assert_eq!(GetCapture(), window.hwnd);
        assert!(matches!(
            window.pop_input(),
            Some(Event::Button {
                button: Button::Left,
                down: true,
                ..
            })
        ));
        SendMessageW(window.hwnd, WM_MOUSEMOVE, None, Some(lp(-10, -10)));
        assert_eq!(window.pop_input(), Some(Event::Move { point: [0.0, 0.0] }));
        SendMessageW(window.hwnd, WM_LBUTTONUP, None, Some(lp(-10, -10)));
        assert!(matches!(
            window.pop_input(),
            Some(Event::Button {
                down: false,
                point: [0.0, 0.0],
                ..
            })
        ));
        assert_ne!(GetCapture(), window.hwnd);
        assert_eq!(window.take_input_request(), None);
        SendMessageW(
            window.hwnd,
            WM_LBUTTONDBLCLK,
            None,
            Some(lp(center[0], center[1])),
        );
        assert!(matches!(
            window.pop_input(),
            Some(Event::Button { double: true, .. })
        ));
        SendMessageW(window.hwnd, WM_CAPTURECHANGED, None, None);
        assert_eq!(window.take_input_request(), Some(false));
        assert_ne!(GetCapture(), window.hwnd);
    }
}
#[test]
fn owned_window_dispatches_live_notifications_paints_and_closes_without_quitting_app() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        assert!(window.is_alive());
        assert!(!IsWindowVisible(window.hwnd).as_bool());
        let mut affinity = 0;
        GetWindowDisplayAffinity(window.hwnd, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        publish(&source);
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(window.hwnd),
            notification::MESSAGE,
            notification::MESSAGE,
            PM_REMOVE
        )
        .as_bool());
        DispatchMessageW(&message);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.take_error().is_none());
        assert!(window.is_alive());
        source.close();
        // Without dispatching close notifications, real WM_PAINT compensating reads must still destroy windows and release subscriptions.
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(!window.is_alive());
        assert!(!source.has_consumers());
        assert!(window.take_error().is_none());
        assert!(window.show().is_err());
        assert!(!PeekMessageW(&mut message, None, WM_QUIT, WM_QUIT, PM_REMOVE).as_bool());
    }
}
#[test]
fn reentrant_notification_is_redrawn_after_outer_host_operation() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        let mut message = MSG::default();
        assert!(PeekMessageW(
            &mut message,
            Some(window.hwnd),
            notification::MESSAGE,
            notification::MESSAGE,
            PM_REMOVE
        )
        .as_bool());
        window
            .state
            .with_host(|_| {
                SendMessageW(
                    window.hwnd,
                    message.message,
                    Some(message.wParam),
                    Some(message.lParam),
                );
                Ok(())
            })
            .unwrap();
        assert!(window.state.deferred.get());
        assert!(reschedule(window.hwnd, &window.state));
        assert!(!window.state.deferred.get());
        // Hidden HWNDs do not guarantee visible update-region maintenance; verify successful single scheduling, then dispatch paint explicitly.
        assert!(!reschedule(window.hwnd, &window.state));
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        // This fixture verifies actual frame collection after reentrancy, not successful display submission by hidden windows.
        assert!(window.frame_for_test().is_some());
        assert!(window.is_alive());
        assert!(window.take_error().is_none());
    }
}
#[test]
#[ignore = "shows only an owned synthetic window; requires an interactive Windows desktop"]
fn actual_minimize_releases_surface_and_restore_reuses_static_frame() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        window.show().unwrap();
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        let has_surface = || {
            window
                .state
                .with_host(|host| Ok(host.has_surface()))
                .unwrap()
        };
        assert_eq!(has_surface(), Some(true));
        for _ in 0..3 {
            let _ = ShowWindow(window.hwnd, SW_SHOWMINNOACTIVE);
            assert!(IsIconic(window.hwnd).as_bool());
            assert_eq!(has_surface(), Some(false));
            // Even explicitly posted paint must not reallocate presentation chains while minimized.
            SendMessageW(window.hwnd, WM_PAINT, None, None);
            assert_eq!(has_surface(), Some(false));
            assert!(source.has_consumers());
            window.show().unwrap();
            assert!(!IsIconic(window.hwnd).as_bool());
            SendMessageW(window.hwnd, WM_PAINT, None, None);
            assert_eq!(has_surface(), Some(true));
            assert!(window.take_error().is_none());
        }
        // Publish only once throughout; recovery must not depend on new frames. Existing source notifications/compensating reads still handle closure.
        let _ = ShowWindow(window.hwnd, SW_SHOWMINNOACTIVE);
        source.close();
        window.refresh().unwrap();
        assert!(!window.is_alive());
        assert!(!source.has_consumers());
        assert!(window.take_error().is_none());
        println!("native-live-output-runtime: minimize=true no-hidden-surface=true static-restore=true cycles=3 close=true");
    }
}
#[test]
fn minimized_size_message_releases_surface_before_next_paint() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.is_alive());
        SendMessageW(
            window.hwnd,
            WM_SIZE,
            Some(WPARAM(SIZE_MINIMIZED as usize)),
            None,
        );
        // Synthetic messages do not actually minimize windows. A new device can rebuild rather than trigger old-surface cross-device rejection,
        // proving WndProc released the presentation owner, without changing any user-window state.
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.is_alive());
        assert!(window.take_error().is_none());
        assert!(source.has_consumers());
    }
}
#[test]
fn incompatible_device_closes_window_and_preserves_one_shot_error() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(window.is_alive());
        assert!(window.take_error().is_none());
        publish(&source);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
        assert!(!window.is_alive());
        assert!(!source.has_consumers());
        assert!(window.take_error().is_some());
        assert!(window.take_error().is_none());
        assert!(!IsWindow(Some(window.hwnd)).as_bool());
    }
}
#[test]
fn owner_relocation_and_user_close_keep_state_safe_and_release_subscription() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::<Texture>::new();
        assert!(LiveWindow::new(source.subscribe().unwrap(), 0, 128).is_err());
        assert!(!source.has_consumers());
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        let pointer = GetWindowLongPtrW(window.hwnd, GWLP_USERDATA);
        let mut moved = Vec::with_capacity(1);
        moved.push(window);
        moved.reserve(10);
        let window = moved.pop().unwrap();
        assert_eq!(GetWindowLongPtrW(window.hwnd, GWLP_USERDATA), pointer);
        SendMessageW(window.hwnd, WM_CLOSE, None, None);
        assert!(!window.is_alive());
        assert!(!source.has_consumers());
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        let hwnd = window.hwnd;
        drop(window);
        assert!(!IsWindow(Some(hwnd)).as_bool());
        assert!(!source.has_consumers());
    }
}

#[test]
#[ignore = "shows an owned synthetic Live window and requires an interactive compositor"]
fn showing_live_repaints_static_frame_without_new_source_delivery() {
    let _apartment = Apartment::new().unwrap();
    unsafe {
        let source = Source::new();
        let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
        publish(&source);
        let frame = window.snapshot_frame().unwrap().unwrap();
        // The frame is already collected; drain notifications so later update regions come only from showing the window, not new source delivery.
        let mut message = MSG::default();
        while PeekMessageW(
            &mut message,
            Some(window.hwnd),
            notification::MESSAGE,
            notification::MESSAGE,
            PM_REMOVE,
        )
        .as_bool()
        {
            DispatchMessageW(&message);
        }
        window.show().unwrap();
        assert!(UpdateWindow(window.hwnd).as_bool());
        assert_eq!(
            window.state.presented.get([128, 128]).unwrap().0,
            frame.sequence
        );
        let _ = ShowWindow(window.hwnd, SW_HIDE);
        window.state.presented.clear();
        let _ = ValidateRect(Some(window.hwnd), None);
        window.show().unwrap();
        assert!(UpdateWindow(window.hwnd).as_bool());
        assert_eq!(
            window.state.presented.get([128, 128]).unwrap().0,
            frame.sequence
        );
        assert!(Arc::ptr_eq(
            &frame,
            &window.snapshot_frame().unwrap().unwrap()
        ));
    }
}
