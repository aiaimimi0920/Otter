//! 无呈现坐标时只退休仍持有输入的会话；不启动 worker/WGC 或操作系统鼠标。
use super::*;
use crate::{desktop::Apartment, live_frames::Source, live_input::Button};

fn fixture(enabled: bool) -> (Source<Texture>, LiveWindow) {
    let frames = Source::new();
    let window = LiveWindow::new(frames.subscribe().unwrap(), 128, 128).unwrap();
    window.allow_input().unwrap();
    window.request_default_input();
    window.input_status(enabled, !enabled, None);
    assert!(viewport(window.hwnd, &window.state).is_none());
    (frames, window)
}
fn send(window: &LiveWindow, message: u32) {
    unsafe {
        handle(
            window.hwnd,
            &window.state,
            message,
            WPARAM(if matches!(message, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) {
                120 << 16
            } else {
                0
            }),
            LPARAM(32 | (32 << 16)),
        )
        .expect("recognized input")
        .unwrap();
    }
}
fn cycle(window: &LiveWindow) -> crate::live_hover_owner::Control {
    window.hover_identity().unwrap().capture_control().unwrap()
}

#[test]
fn enabled_main_and_popup_pointer_activity_without_a_frame_keeps_the_control_cycle() {
    let _apartment = Apartment::new().unwrap();
    for popup in [false, true] {
        let (_frames, window) = fixture(true);
        window.state.popup.set(popup);
        assert_eq!(window.take_input_request(), Some(true));
        let control = cycle(&window);
        for message in [WM_MOUSEMOVE, WM_MOUSEHOVER, WM_MOUSEMOVE] {
            send(&window, message);
            let input = window.state.input.borrow();
            assert!(input.enabled && input.desired && !input.pending);
            assert!(input.events.is_empty());
            assert!(
                control.valid(),
                "missing presentation cannot revoke parent scopes"
            );
            assert_eq!(cycle(&window).epoch(), control.epoch());
        }
        assert_eq!(window.take_input_request(), None);
    }
}

#[test]
fn pending_pointer_activity_without_a_frame_preserves_the_enable_request() {
    let _apartment = Apartment::new().unwrap();
    let (_frames, window) = fixture(false);
    let control = cycle(&window);
    send(&window, WM_MOUSEHOVER);
    assert!(control.valid());
    let input = window.state.input.borrow();
    assert!(input.desired && input.pending && !input.enabled);
    assert!(input.events.is_empty());
    drop(input);
    assert_eq!(window.take_input_request(), Some(true));
}

#[test]
fn unmapped_buttons_and_wheels_are_not_queued_or_latched() {
    let _apartment = Apartment::new().unwrap();
    let (_frames, window) = fixture(true);
    // 附属窗绕过本地 Alt-drag/缩放手势；本回归不依赖全局修饰键状态。
    window.state.popup.set(true);
    window.take_input_request();
    let control = cycle(&window);
    for message in [
        WM_LBUTTONDOWN,
        WM_LBUTTONUP,
        WM_RBUTTONDOWN,
        WM_RBUTTONUP,
        WM_RBUTTONDBLCLK,
        WM_RBUTTONUP,
        WM_MBUTTONDOWN,
        WM_MBUTTONUP,
        WM_MOUSEWHEEL,
        WM_MOUSEHWHEEL,
    ] {
        send(&window, message);
        let input = window.state.input.borrow();
        assert!(input.events.is_empty());
        assert_eq!(input.buttons, 0);
        assert!(input.pointer.is_none());
        assert!(input.enabled && control.valid());
    }
    assert_eq!(window.take_input_request(), None);
}

#[test]
fn missing_mapping_with_a_held_mouse_or_key_still_requests_original_cleanup() {
    let _apartment = Apartment::new().unwrap();
    let mouse = Event::Button {
        point: [0.25, 0.25],
        button: Button::Right,
        down: true,
        double: false,
    };
    let key = Event::Key {
        code: 65,
        down: true,
    };
    for held in [mouse, key] {
        let (_frames, window) = fixture(true);
        window.take_input_request();
        let control = cycle(&window);
        {
            let mut input = window.state.input.borrow_mut();
            if held == mouse {
                assert!(input.button_edge(2, true));
            }
            assert!(input.push(held));
        }
        send(&window, WM_MOUSEHOVER);
        assert_eq!(window.take_input_request(), Some(false));
        assert!(!control.valid());
        let input = window.state.input.borrow();
        assert!(input.allowed && input.desired && !input.failed);
        assert!(!input.enabled && !input.pending);
        assert_eq!(input.buttons, 0);
        assert_eq!(input.held_keys, [0; 4]);
        assert!(input.events.is_empty());
    }
}

#[test]
fn already_handed_off_releases_keep_their_fifo_and_cycle_without_a_frame() {
    let _apartment = Apartment::new().unwrap();
    let (_frames, window) = fixture(true);
    window.take_input_request();
    let control = cycle(&window);
    let expected = {
        let mut input = window.state.input.borrow_mut();
        assert!(input.button_edge(2, true));
        assert!(input.push(Event::Button {
            point: [0.25, 0.25],
            button: Button::Right,
            down: true,
            double: false,
        }));
        assert!(input.push(Event::Key {
            code: 65,
            down: true
        }));
        assert!(input.handoff_source());
        assert_eq!(input.buttons, 0);
        assert_eq!(input.held_keys, [0; 4]);
        assert_eq!(input.events.len(), 4);
        input.events.clone()
    };
    send(&window, WM_MOUSEHOVER);
    assert!(control.valid());
    let input = window.state.input.borrow();
    assert!(input.enabled && input.desired);
    assert_eq!(
        input.events, expected,
        "worker still owns the queued releases"
    );
    drop(input);
    assert_eq!(window.take_input_request(), None);
}
