//! 真实隐藏 HWND 的滚动消息及子 EDIT 几何同步；不注入系统输入或改系统偏好。
use super::*;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
#[path = "surface_window_gesture_tests.rs"]
mod gesture_tests;
#[path = "surface_window_scroll_key_tests.rs"]
mod key_tests;
#[path = "surface_window_repeat_tests.rs"]
mod repeat_tests;
#[path = "surface_window_scrollbar_tests.rs"]
mod scrollbar_tests;
#[path = "surface_select_scroll_tests.rs"]
mod select_tests;
#[path = "surface_slider_scroll_probe_fixture.rs"]
pub(in crate::desktop) mod slider_scroll_probe_fixture;
fn scene() -> Scene {
    Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"space","type":"view","layout":{"height":80,"minHeight":80}},{"id":"field","type":"input","props":{"value":"draft"},"layout":{"height":32,"minHeight":32}},{"id":"tail","type":"view","layout":{"height":200,"minHeight":200}}]}"#).unwrap()
}
#[test]
fn surface_scroll_hwnd_moves_existing_edit_without_resetting_draft() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let child = FindWindowExW(Some(window.hwnd), None, w!("EDIT"), None).unwrap();
        SetWindowTextW(child, w!("local draft")).unwrap();
        let mut before = RECT::default();
        GetWindowRect(child, &mut before).unwrap();
        window
            .state
            .run(|host| {
                assert!(host.scroll([5, 5], 1, 60., false)?);
                Ok(())
            })
            .unwrap();
        let same = FindWindowExW(Some(window.hwnd), None, w!("EDIT"), None).unwrap();
        assert_eq!(child, same);
        let mut after = RECT::default();
        GetWindowRect(child, &mut after).unwrap();
        let expected = (60. * GetDpiForWindow(window.hwnd) as f32 / 96.).round() as i32;
        assert!((before.top - after.top - expected).abs() <= 1);
        let mut text = [0u16; 64];
        let count = GetWindowTextW(child, &mut text);
        assert_eq!(
            String::from_utf16(&text[..count as usize]).unwrap(),
            "local draft"
        );
    }
    assert!(window.take_error().is_none());
}
#[test]
fn surface_scroll_wheel_respects_local_gate_and_system_preference() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    unsafe {
        SetWindowPos(
            window.hwnd,
            None,
            10,
            10,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .unwrap();
        let mut point = POINT { x: 5, y: 5 };
        assert!(ClientToScreen(window.hwnd, &mut point).as_bool());
        let lp = LPARAM(((point.x as u16 as u32) | ((point.y as u16 as u32) << 16)) as isize);
        let wp = WPARAM((-120i16 as u16 as usize) << 16);
        let origin = || {
            window
                .state
                .host
                .borrow()
                .as_ref()
                .unwrap()
                .presentation()
                .geometry()
                .nodes()[1]
                .origin()[1]
        };
        SendMessageW(window.hwnd, WM_MOUSEWHEEL, Some(wp), Some(lp));
        assert_eq!(origin(), 0.);
        window.set_input_policy(true, false).unwrap();
        let mut lines = 3u32;
        if SystemParametersInfoW(
            SPI_GETWHEELSCROLLLINES,
            0,
            Some((&mut lines as *mut u32).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
        )
        .is_err()
        {
            lines = 3;
        }
        SendMessageW(window.hwnd, WM_MOUSEWHEEL, Some(wp), Some(lp));
        if lines == 0 {
            assert_eq!(origin(), 0.);
        } else {
            assert!(origin() < 0.);
        }
        let after = origin();
        window.set_input_policy(false, false).unwrap();
        SendMessageW(window.hwnd, WM_MOUSEWHEEL, Some(wp), Some(lp));
        assert_eq!(origin(), after);
    }
    assert!(window.take_error().is_none());
}
