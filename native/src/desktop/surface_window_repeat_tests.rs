//! 真实 HWND + 可控单调 tick；不睡眠或注入全局输入来伪造长按时序。
use super::*;
use std::time::{Duration, Instant};
use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
#[test]
fn surface_scroll_repeat_real_timer_reaches_window_dispatch_and_stops_on_release() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(long_scene(), 240, 200).unwrap();
    window.set_input_policy(true, false).unwrap();
    let start = Instant::now();
    unsafe {
        send(&window, WM_LBUTTONDOWN, [95., 90.]);
        while offset(&window) < 200. {
            let mut message = MSG::default();
            while PeekMessageW(
                &mut message,
                Some(window.hwnd),
                WM_TIMER,
                WM_TIMER,
                PM_REMOVE,
            )
            .as_bool()
            {
                DispatchMessageW(&message);
            }
            assert!(start.elapsed() < Duration::from_secs(3));
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(start.elapsed() >= Duration::from_millis(400));
        let old = id(&window);
        send(&window, WM_LBUTTONUP, [95., 90.]);
        let stopped = offset(&window);
        SendMessageW(window.hwnd, WM_TIMER, Some(WPARAM(old)), None);
        assert_eq!(offset(&window), stopped);
    }
    assert!(window.state.scroll_repeat.borrow().is_none());
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
fn long_scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"content","type":"view","layout":{"height":2000,"minHeight":2000}}]}"#).unwrap()
}
fn offset(window: &SurfaceWindow) -> f32 {
    -window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .geometry()
        .nodes()[1]
        .origin()[1]
}
unsafe fn send(window: &SurfaceWindow, message: u32, point: [f32; 2]) {
    let scale = GetDpiForWindow(window.hwnd) as f32 / 96.;
    let point = point.map(|v| (v * scale).round() as i32);
    SendMessageW(
        window.hwnd,
        message,
        None,
        Some(LPARAM(
            (point[0] as u16 as usize | ((point[1] as u16 as usize) << 16)) as isize,
        )),
    );
}
fn id(window: &SurfaceWindow) -> usize {
    window.state.scroll_repeat.borrow().as_ref().unwrap().id()
}
fn tick(window: &SurfaceWindow, id: usize, time: Instant) -> bool {
    repeat::timer(window.hwnd, &window.state, id, time)
}
#[test]
fn surface_scroll_repeat_delays_bounds_ticks_and_pauses_outside_track() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(long_scene(), 240, 200).unwrap();
    window.set_input_policy(true, false).unwrap();
    let start = Instant::now();
    unsafe {
        send(&window, WM_LBUTTONDOWN, [95., 90.]);
    }
    let id = id(&window);
    assert_eq!(offset(&window), 100.);
    assert!(tick(&window, id, start));
    assert_eq!(offset(&window), 100.);
    let after = start + Duration::from_secs(1);
    assert!(tick(&window, id, after));
    assert_eq!(offset(&window), 200.);
    assert!(tick(&window, id, after));
    assert_eq!(offset(&window), 200.);
    // 一次晚到的 tick 只翻一页，不追赶积压的 20 次间隔。
    assert!(tick(&window, id, after + Duration::from_secs(1)));
    assert_eq!(offset(&window), 300.);
    unsafe {
        send(&window, WM_MOUSEMOVE, [5., 90.]);
    }
    assert!(tick(&window, id, after + Duration::from_secs(2)));
    assert_eq!(offset(&window), 300.);
    unsafe {
        send(&window, WM_MOUSEMOVE, [95., 90.]);
    }
    assert!(tick(&window, id, after + Duration::from_secs(3)));
    assert_eq!(offset(&window), 400.);
    unsafe {
        send(&window, WM_LBUTTONUP, [95., 90.]);
    }
    assert!(window.state.scroll_repeat.borrow().is_none());
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    assert!(!tick(&window, id, after + Duration::from_secs(4)));
    assert_eq!(offset(&window), 400.);
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_scroll_repeat_stops_at_thumb_and_old_timer_cannot_drive_new_gesture() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(long_scene(), 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDOWN, [95., 50.]);
    }
    let first = id(&window);
    let now = Instant::now();
    for step in 1..40 {
        assert!(tick(&window, first, now + Duration::from_secs(step)));
    }
    let stopped = offset(&window);
    assert!(stopped > 100. && stopped < 1900.);
    assert!(tick(&window, first, now + Duration::from_secs(50)));
    assert_eq!(offset(&window), stopped);
    unsafe {
        send(&window, WM_LBUTTONUP, [95., 50.]);
        send(&window, WM_LBUTTONDOWN, [95., 95.]);
    }
    let second = id(&window);
    assert_ne!(first, second);
    let current = offset(&window);
    assert!(!tick(&window, first, now + Duration::from_secs(60)));
    assert_eq!(offset(&window), current);
    assert!(tick(&window, second, now + Duration::from_secs(60)));
    assert!(offset(&window) > current);
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_scroll_repeat_lifecycle_cancels_capture_and_timer_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(long_scene(), 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    for message in [
        WM_CANCELMODE,
        WM_KILLFOCUS,
        WM_SIZE,
        WM_CAPTURECHANGED,
        WM_DPICHANGED,
    ] {
        unsafe {
            send(&window, WM_LBUTTONDOWN, [95., 95.]);
        }
        let old = id(&window);
        unsafe {
            SendMessageW(window.hwnd, message, None, None);
        }
        assert!(window.state.scroll_repeat.borrow().is_none());
        assert_ne!(unsafe { GetCapture() }, window.hwnd);
        assert!(!tick(&window, old, Instant::now() + Duration::from_secs(1)));
    }
    unsafe {
        send(&window, WM_LBUTTONDOWN, [95., 95.]);
    }
    window.replace(long_scene()).unwrap();
    assert!(window.state.scroll_repeat.borrow().is_none());
    unsafe {
        send(&window, WM_LBUTTONDOWN, [95., 95.]);
    }
    window.set_input_policy(false, false).unwrap();
    assert!(window.state.scroll_repeat.borrow().is_none());
    window.set_input_policy(true, false).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDOWN, [95., 95.]);
    }
    window.close().unwrap();
    assert!(window.state.scroll_repeat.borrow().is_none());
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
}
