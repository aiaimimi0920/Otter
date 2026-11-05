use super::*;
#[path = "surface_window_foreground_tests.rs"]
mod foreground_tests;
#[path = "surface_window_grid_tests.rs"]
mod grid_tests;
#[path = "surface_window_image_tests.rs"]
mod image_tests;
#[path = "surface_window_javascript_tests.rs"]
mod javascript_tests;
#[test]
fn surface_icon_creates_and_closes_real_native_window() {
    let _com = super::super::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"icon","type":"icon","props":{"glyph":"X"}}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 200).unwrap();
    window.close().unwrap();
    assert!(window.take_error().is_none());
}
#[path = "surface_window_edit_tests.rs"]
mod edit_tests;
#[path = "surface_window_pointer_tests.rs"]
mod pointer_tests;
fn scene(id: &str) -> Scene {
    Scene::parse(
        format!(r#"{{"id":"{id}","type":"text","props":{{"text":"Native Surface"}}}}"#).as_bytes(),
    )
    .unwrap()
}
#[test]
fn window_prepares_current_pixel_target_and_rejects_busy_or_closed_state() {
    let _apartment = super::super::Apartment::new().unwrap();
    let binding = crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap();
    let scene = Scene::parse(
        br#"{"id":"root","type":"view","layout":{"height":"100%"},"events":{"click":"run"}}"#,
    )
    .unwrap();
    let window = SurfaceWindow::new(scene, 200, 140).unwrap();
    let events = window
        .click_events([1, 1], &binding, 9, Default::default())
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].revision(), 9);
    assert!(!events[0].attempted());
    assert!(window
        .click_events([-1, 1], &binding, 9, Default::default())
        .unwrap()
        .is_empty());
    let host = window.state.host.borrow_mut().take();
    assert!(window
        .click_events([1, 1], &binding, 9, Default::default())
        .is_err());
    *window.state.host.borrow_mut() = host;
    window.close().unwrap();
    assert!(window
        .click_events([1, 1], &binding, 9, Default::default())
        .is_err());
}
#[test]
fn window_hit_rejects_busy_and_closed_state_without_stale_target() {
    let _apartment = super::super::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene("target"), 200, 140).unwrap();
    assert_eq!(window.hit_test([1, 1]).unwrap().as_deref(), Some("target"));
    let host = window.state.host.borrow_mut().take();
    assert!(window.hit_test([1, 1]).is_err());
    *window.state.host.borrow_mut() = host;
    window.close().unwrap();
    assert!(window.hit_test([1, 1]).is_err());
}
#[test]
fn hidden_surface_window_dispatches_paint_replace_resize_and_close() {
    let _apartment = super::super::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene("first"), 200, 140).unwrap();
    assert!(window.is_alive());
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    window.replace(scene("second")).unwrap();
    unsafe {
        SetWindowPos(
            window.hwnd,
            None,
            0,
            0,
            280,
            180,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOZORDER,
        )
        .unwrap();
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    assert_eq!(
        window
            .state
            .host
            .borrow()
            .as_ref()
            .unwrap()
            .presentation()
            .scene()
            .root()
            .id(),
        "second"
    );
    let hwnd = window.hwnd;
    // 模拟 D2D 调用期间的同步关闭消息：取出的 Host 不能被放回已关闭状态。
    assert!(window
        .state
        .run(|_| {
            unsafe {
                SendMessageW(hwnd, WM_CLOSE, None, None);
            }
            Ok(())
        })
        .unwrap());
    assert!(!window.is_alive());
    assert!(!unsafe { IsWindow(Some(hwnd)) }.as_bool());
    assert!(window.state.host.borrow().is_none());
    assert!(window.replace(scene("closed")).is_err());
    window.close().unwrap();
}

#[test]
fn dpi_message_applies_suggested_bounds_and_rejects_overflow() {
    let _apartment = super::super::Apartment::new().unwrap();
    let window = SurfaceWindow::new(scene("dpi"), 200, 140).unwrap();
    let suggested = RECT {
        left: 40,
        top: 50,
        right: 360,
        bottom: 270,
    };
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_DPICHANGED,
            Some(WPARAM(144 | (144 << 16))),
            Some(LPARAM((&suggested as *const RECT) as isize)),
        );
        let mut actual = RECT::default();
        GetWindowRect(window.hwnd, &mut actual).unwrap();
        assert_eq!(actual, suggested);
        let invalid = RECT {
            left: i32::MIN,
            top: 0,
            right: i32::MAX,
            bottom: 100,
        };
        SendMessageW(
            window.hwnd,
            WM_DPICHANGED,
            None,
            Some(LPARAM((&invalid as *const RECT) as isize)),
        );
        GetWindowRect(window.hwnd, &mut actual).unwrap();
        assert_eq!(actual, suggested);
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
    assert!(!unsafe { IsWindowVisible(window.hwnd) }.as_bool());
}
