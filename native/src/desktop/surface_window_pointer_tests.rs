//! 隐藏 HWND 的实际鼠标消息/捕获测试；只生成候选，不发送任何动作。
use super::*;
#[path = "surface_window_feedback_tests.rs"]
mod feedback_tests;
#[path = "surface_window_keyboard_tests.rs"]
mod keyboard_tests;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture};
fn window() -> SurfaceWindow {
    SurfaceWindow::new(
        Scene::parse(
            br#"{"id":"target","type":"view","layout":{"height":"100%"},"events":{"click":"run"}}"#,
        )
        .unwrap(),
        240,
        160,
    )
    .unwrap()
}
fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
#[test]
fn surface_button_window_click_routes_action_without_bubbling_and_respects_disabled() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    for disabled in [false, true] {
        let json = format!(
            r#"{{"id":"root","type":"view","events":{{"click":"outer"}},"children":[{{"id":"button","type":"button","props":{{"label":"Run","disabled":{disabled}}},"layout":{{"width":120,"height":60}},"events":{{"click":"run"}}}}]}}"#
        );
        let window = SurfaceWindow::new(Scene::parse(json.as_bytes()).unwrap(), 240, 160).unwrap();
        window.set_pointer_enabled(true).unwrap();
        unsafe {
            SendMessageW(window.hwnd, WM_PAINT, None, None);
            assert!(window.take_error().is_none());
            let candidate = click(&window);
            assert_eq!(candidate.node_id, "button");
            let events = window
                .click_events_for_candidate(candidate, &binding(), 7)
                .unwrap();
            assert_eq!(events.len(), usize::from(!disabled));
            if !disabled {
                let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
                assert_eq!(wire["action"], "run");
                assert_eq!(events[0].revision(), 7);
                assert!(!events[0].attempted());
            }
        }
    }
}
#[test]
fn unchanged_pointer_policy_preserves_press_and_queued_candidate() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let window = window();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 10, 10);
        window.set_pointer_enabled(true).unwrap();
        send(&window, WM_LBUTTONUP, 10, 10);
        assert!(window.has_pending_click());
        window.set_pointer_enabled(true).unwrap();
        assert!(window.has_pending_click());
        window.set_pointer_enabled(false).unwrap();
        assert!(!window.pointer_enabled());
        assert!(!window.has_pending_click());
    }
}
unsafe fn click(window: &SurfaceWindow) -> Click {
    send(window, WM_LBUTTONDOWN, 10, 10);
    send(window, WM_LBUTTONUP, 10, 10);
    window.take_click().unwrap()
}
#[test]
fn consumed_candidate_cannot_cross_window_or_survive_scene_replacement() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let first = window();
    let second = window();
    first.set_pointer_enabled(true).unwrap();
    second.set_pointer_enabled(true).unwrap();
    unsafe {
        let candidate = click(&first);
        assert!(second
            .click_events_for_candidate(candidate, &binding(), 1)
            .is_err());
        let candidate = click(&first);
        // 复用同一个节点 ID 但替换动作，旧点击仍必须失效。
        first.replace(Scene::parse(br#"{"id":"target","type":"view","layout":{"height":"100%"},"events":{"click":"changed"}}"#).unwrap()).unwrap();
        assert!(first
            .click_events_for_candidate(candidate, &binding(), 1)
            .is_err());
        let candidate = click(&first);
        let events = first
            .click_events_for_candidate(candidate, &binding(), 2)
            .unwrap();
        assert_eq!(events.len(), 1);
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["action"], "changed");
        assert_eq!(events[0].revision(), 2);
        assert!(!events[0].attempted());
        assert!(first.take_click().is_none());
    }
}
#[test]
fn consumed_candidate_is_invalid_after_resize_focus_loss_or_disable() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let window = window();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        for message in [WM_SIZE, WM_KILLFOCUS, WM_CANCELMODE] {
            let candidate = click(&window);
            send(&window, message, 0, 0);
            assert!(window
                .click_events_for_candidate(candidate, &binding(), 1)
                .is_err());
        }
        let candidate = click(&window);
        window.set_pointer_enabled(false).unwrap();
        assert!(window
            .click_events_for_candidate(candidate, &binding(), 1)
            .is_err());
    }
}
unsafe fn send(window: &SurfaceWindow, message: u32, x: i16, y: i16) {
    let value = (x as u16 as u32) | ((y as u16 as u32) << 16);
    SendMessageW(window.hwnd, message, None, Some(LPARAM(value as isize)));
}
#[test]
fn pointer_is_opt_in_single_slot_and_releases_capture_on_click() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let window = window();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 10, 10);
        send(&window, WM_LBUTTONUP, 10, 10);
        assert!(window.take_click().is_none());
        window.set_pointer_enabled(true).unwrap();
        send(&window, WM_LBUTTONDOWN, 10, 10);
        assert_eq!(GetCapture(), window.hwnd);
        send(&window, WM_LBUTTONUP, 10, 10);
        assert_ne!(GetCapture(), window.hwnd);
        // 第二个点击不能覆盖尚未消费的第一个。
        send(&window, WM_LBUTTONDOWN, 20, 20);
        send(&window, WM_LBUTTONUP, 20, 20);
        let click = window.take_click().unwrap();
        assert_eq!(click.client_pixels, [10, 10]);
        assert_eq!(click.node_id, "target");
        assert!(window.take_click().is_none());
    }
}
#[test]
fn drag_focus_capture_loss_and_negative_release_cancel_click() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let window = window();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        for cancellation in [WM_MOUSEMOVE, WM_KILLFOCUS, WM_CANCELMODE] {
            send(&window, WM_LBUTTONDOWN, 10, 10);
            send(&window, cancellation, 120, 80);
            send(&window, WM_LBUTTONUP, 10, 10);
            assert!(window.take_click().is_none());
            assert_ne!(GetCapture(), window.hwnd);
        }
        send(&window, WM_LBUTTONDOWN, 10, 10);
        ReleaseCapture().unwrap();
        send(&window, WM_LBUTTONUP, 10, 10);
        assert!(window.take_click().is_none());
        send(&window, WM_LBUTTONDOWN, 10, 10);
        send(&window, WM_LBUTTONUP, -1, -1);
        assert!(window.take_click().is_none());
        send(&window, WM_LBUTTONDOWN, 10, 10);
        send(&window, WM_LBUTTONUP, 120, 80);
        assert!(window.take_click().is_none());
    }
}
#[test]
fn replacement_disable_resize_and_close_clear_pending_pointer_state() {
    let _apartment = super::super::super::Apartment::new().unwrap();
    let window = window();
    window.set_pointer_enabled(true).unwrap();
    unsafe {
        send(&window, WM_LBUTTONDOWN, 10, 10);
        window.replace(scene("new")).unwrap();
        send(&window, WM_LBUTTONUP, 10, 10);
        assert!(window.take_click().is_none());
        send(&window, WM_LBUTTONDOWN, 1, 1);
        send(&window, WM_LBUTTONUP, 1, 1);
        window.set_pointer_enabled(false).unwrap();
        assert!(window.take_click().is_none());
        window.set_pointer_enabled(true).unwrap();
        send(&window, WM_LBUTTONDOWN, 1, 1);
        send(&window, WM_SIZE, 0, 0);
        send(&window, WM_LBUTTONUP, 1, 1);
        assert!(window.take_click().is_none());
        send(&window, WM_LBUTTONDOWN, 1, 1);
        window.close().unwrap();
        assert_ne!(GetCapture(), window.hwnd);
        assert!(window.take_click().is_none());
        assert!(window.set_pointer_enabled(true).is_err());
    }
}
