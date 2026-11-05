//! 真实 HWND 滚动条捕获、权限撤销和 EDIT 遮挡；不注入全局鼠标。
use super::*;
use windows::Win32::UI::Input::KeyboardAndMouse::GetCapture;
#[test]
fn surface_scrollbar_double_click_keeps_scroll_capture_priority() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDBLCLK, [95., 10.]);
        assert_eq!(GetCapture(), window.hwnd);
        send(&window, WM_MOUSEMOVE, [95., 40.]);
        assert!(y(&window) < 0.);
        send(&window, WM_LBUTTONUP, [95., 40.]);
        assert_ne!(GetCapture(), window.hwnd);
    }
    assert!(window.state.scroll_drag.borrow().is_none());
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_scrollbar_hwnd_busy_host_cannot_create_drag_or_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    let host = window.state.host.borrow_mut().take();
    unsafe {
        send(&window, WM_LBUTTONDOWN, [95., 10.]);
    }
    assert!(window.state.scroll_drag.borrow().is_none());
    assert!(window.take_click().is_none());
    assert_ne!(unsafe { GetCapture() }, window.hwnd);
    *window.state.host.borrow_mut() = host;
    assert!(window.take_error().is_none());
}
unsafe fn send(window: &SurfaceWindow, message: u32, p: [f32; 2]) {
    let scale = GetDpiForWindow(window.hwnd) as f32 / 96.;
    let p = p.map(|v| (v * scale).round() as i32);
    let lp = LPARAM((p[0] as u16 as usize | ((p[1] as u16 as usize) << 16)) as isize);
    SendMessageW(window.hwnd, message, Some(WPARAM(0)), Some(lp));
}
fn y(window: &SurfaceWindow) -> f32 {
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
}
#[test]
fn surface_scrollbar_hwnd_does_not_swallow_content_release() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDOWN, [5., 10.]);
        assert_eq!(GetCapture(), window.hwnd);
        send(&window, WM_MOUSEMOVE, [95., 10.]);
        send(&window, WM_LBUTTONUP, [95., 10.]);
        assert_ne!(GetCapture(), window.hwnd);
        assert!(window.take_click().is_none());
        assert!(window.state.scroll_drag.borrow().is_none());
    }
}
#[test]
fn surface_scrollbar_hwnd_drag_survives_moves_and_never_queues_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDOWN, [95., 10.]);
        send(&window, WM_MOUSEMOVE, [95., 60.]);
        assert_eq!(y(&window), 0.);
        window.set_input_policy(true, false).unwrap();
        send(&window, WM_LBUTTONDOWN, [95., 10.]);
        assert_eq!(GetCapture(), window.hwnd);
        send(&window, WM_MOUSEMOVE, [95., 40.]);
        assert!(y(&window) < 0.);
        let first = y(&window);
        assert_eq!(GetCapture(), window.hwnd);
        send(&window, WM_MOUSEMOVE, [95., 60.]);
        assert!(y(&window) < first);
        send(&window, WM_LBUTTONUP, [95., 60.]);
        assert_ne!(GetCapture(), window.hwnd);
        assert!(window.state.scroll_drag.borrow().is_none());
        assert!(window.take_click().is_none());
        assert!(window.take_error().is_none());
    }
}
#[test]
fn surface_scrollbar_hwnd_cancel_replace_resize_and_policy_end_capture() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        for message in [WM_CANCELMODE, WM_KILLFOCUS, WM_SIZE, WM_CAPTURECHANGED] {
            send(&window, WM_LBUTTONDOWN, [95., 10.]);
            assert!(window.state.scroll_drag.borrow().is_some());
            SendMessageW(window.hwnd, message, None, None);
            assert!(window.state.scroll_drag.borrow().is_none());
            assert_ne!(GetCapture(), window.hwnd);
            send(&window, WM_MOUSEMOVE, [95., 70.]);
            assert_eq!(y(&window), 0.);
        }
        send(&window, WM_LBUTTONDOWN, [95., 10.]);
        window.replace(scene()).unwrap();
        assert!(window.state.scroll_drag.borrow().is_none());
        send(&window, WM_LBUTTONDOWN, [95., 10.]);
        window.set_input_policy(false, false).unwrap();
        send(&window, WM_MOUSEMOVE, [95., 70.]);
        assert_eq!(y(&window), 0.);
        assert!(window.take_click().is_none());
        assert_ne!(GetCapture(), window.hwnd);
    }
    assert!(window.take_error().is_none());
}
#[test]
fn surface_scrollbar_hwnd_track_pages_and_edit_region_excludes_overlay() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene(), 240, 200).unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let child = FindWindowExW(Some(window.hwnd), None, w!("EDIT"), None).unwrap();
        SetWindowTextW(child, w!("local draft")).unwrap();
        let region = CreateRectRgn(0, 0, 0, 0);
        assert_ne!(GetWindowRgn(child, region), RGN_ERROR);
        let scale = GetDpiForWindow(window.hwnd) as f32 / 96.;
        assert!(PtInRegion(region, (5. * scale) as i32, (5. * scale) as i32).as_bool());
        assert!(!PtInRegion(region, (95. * scale) as i32, (5. * scale) as i32).as_bool());
        let _ = DeleteObject(region.into());
        send(&window, WM_LBUTTONDOWN, [95., 80.]);
        send(&window, WM_LBUTTONUP, [95., 80.]);
        assert_eq!(y(&window), -100.);
        assert!(window.take_click().is_none());
        assert_eq!(
            FindWindowExW(Some(window.hwnd), None, w!("EDIT"), None).unwrap(),
            child
        );
        let mut text = [0u16; 32];
        let count = GetWindowTextW(child, &mut text);
        assert_eq!(
            String::from_utf16(&text[..count as usize]).unwrap(),
            "local draft"
        );
    }
    assert!(window.take_error().is_none());
}
