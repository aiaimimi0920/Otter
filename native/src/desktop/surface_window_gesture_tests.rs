//! 真实 HWND 配置读回 + 合成系统手势数据；不伪称触屏硬件或真实惯性验收。
use super::*;
use windows::Win32::System::SystemServices::*;
use windows::Win32::UI::Input::Touch::*;
fn window() -> SurfaceWindow {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"content","type":"view","layout":{"width":300,"minWidth":300,"height":300,"minHeight":300},"children":[{"id":"text","type":"text","props":{"text":"hello"}}]}]}"#).unwrap();
    SurfaceWindow::new(scene, 240, 200).unwrap()
}
fn position(window: &SurfaceWindow) -> [f32; 2] {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .presentation()
        .geometry()
        .nodes()[1]
        .origin()
}
fn pan(window: &SurfaceWindow, sequence: u32, flags: u32, logical: [f32; 2]) {
    let scale = unsafe { GetDpiForWindow(window.hwnd) } as f32 / 96.;
    gesture::pan(
        window.hwnd,
        &window.state,
        sequence,
        flags,
        logical.map(|v| (v * scale).round() as i32),
    );
}
fn near(actual: [f32; 2], expected: [f32; 2]) {
    for (a, e) in actual.into_iter().zip(expected) {
        assert!((a - e).abs() < 1.1, "{actual:?} != {expected:?}");
    }
}
#[test]
fn surface_gesture_hwnd_requests_single_finger_axes_and_system_inertia() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    let mut count = 1;
    let mut config = GESTURECONFIG {
        dwID: GID_PAN,
        ..Default::default()
    };
    unsafe {
        GetGestureConfig(
            window.hwnd,
            0,
            0,
            &raw mut count,
            &mut config,
            std::mem::size_of::<GESTURECONFIG>() as u32,
        )
        .unwrap();
    }
    assert_eq!(
        config.dwWant
            & (GC_PAN
                | GC_PAN_WITH_SINGLE_FINGER_HORIZONTALLY
                | GC_PAN_WITH_SINGLE_FINGER_VERTICALLY
                | GC_PAN_WITH_INERTIA)
                .0,
        23
    );
    assert_eq!(config.dwBlock & GC_PAN_WITH_GUTTER.0, GC_PAN_WITH_GUTTER.0);
}
#[test]
fn surface_gesture_pan_inertia_end_and_sequence_guard_share_cached_frame() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    pan(&window, 7, GF_BEGIN, [60., 60.]);
    assert!(window.state.pan.borrow().is_none());
    window.set_input_policy(true, false).unwrap();
    let measures = crate::surface_text::measure_calls();
    pan(&window, 7, GF_BEGIN, [60., 60.]);
    assert_eq!(position(&window), [0., 0.]);
    pan(&window, 8, GF_INERTIA, [0., 0.]);
    assert_eq!(position(&window), [0., 0.]);
    pan(&window, 7, 0, [40., 30.]);
    near(position(&window), [-20., -30.]);
    pan(&window, 7, GF_INERTIA, [20., 10.]);
    near(position(&window), [-40., -50.]);
    pan(&window, 7, GF_INERTIA | GF_END, [10., 0.]);
    near(position(&window), [-50., -60.]);
    assert!(window.state.pan.borrow().is_none());
    pan(&window, 7, GF_INERTIA, [0., -20.]);
    near(position(&window), [-50., -60.]);
    assert_eq!(crate::surface_text::measure_calls(), measures);
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_gesture_cancel_policy_scene_resize_and_busy_do_not_replay_pan() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = window();
    window.set_input_policy(true, true).unwrap();
    for message in [
        WM_CANCELMODE,
        WM_KILLFOCUS,
        WM_CAPTURECHANGED,
        WM_SIZE,
        WM_DPICHANGED,
    ] {
        pan(&window, 7, GF_BEGIN, [60., 60.]);
        unsafe {
            SendMessageW(window.hwnd, message, None, None);
        }
        pan(&window, 7, GF_INERTIA, [0., 0.]);
        assert_eq!(position(&window), [0., 0.]);
        assert!(window.state.pan.borrow().is_none());
    }
    pan(&window, 7, GF_BEGIN, [60., 60.]);
    let host = window.state.host.borrow_mut().take();
    pan(&window, 7, 0, [40., 40.]);
    *window.state.host.borrow_mut() = host;
    pan(&window, 7, 0, [30., 30.]);
    near(position(&window), [-10., -10.]);
    window.set_input_policy(false, false).unwrap();
    pan(&window, 7, GF_INERTIA, [0., 0.]);
    near(position(&window), [-10., -10.]);
    assert!(window.state.pan.borrow().is_none());
    window.set_input_policy(true, true).unwrap();
    pan(&window, 9, GF_BEGIN, [60., 60.]);
    window
        .replace(Scene::parse(br#"{"id":"replacement","type":"view"}"#).unwrap())
        .unwrap();
    assert!(window.state.pan.borrow().is_none());
    pan(&window, 10, GF_BEGIN, [5., 5.]);
    window.close().unwrap();
    assert!(window.state.pan.borrow().is_none());
    pan(&window, 10, GF_INERTIA, [0., 0.]);
    assert!(window.state.pan.borrow().is_none());
    assert!(window.take_error().is_none());
}
