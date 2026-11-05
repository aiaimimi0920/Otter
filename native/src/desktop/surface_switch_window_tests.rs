//! 真实隐藏 HWND 的 Switch 激活/共享编辑 FIFO；不注入桌面输入或发送网络动作。
use super::*;
#[path = "surface_switch_bubble_tests.rs"]
mod bubble;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn scene(value: bool, disabled: bool, action: &str) -> Scene {
    Scene::parse(serde_json::json!({"id":"switch","type":"switch","props":{"value":value,"disabled":disabled},"events":{"change":action}}).to_string().as_bytes()).unwrap()
}
fn mount() -> SurfaceWindow {
    let window = SurfaceWindow::new(scene(false, false, "change"), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window
}
fn value(window: &SurfaceWindow) -> bool {
    window
        .state
        .host
        .borrow()
        .as_ref()
        .unwrap()
        .switch_value("switch")
        .unwrap()
}
fn mouse(window: &SurfaceWindow, message: u32, x: u16, y: u16) {
    unsafe {
        SendMessageW(
            window.hwnd,
            message,
            None,
            Some(LPARAM(((y as usize) << 16 | x as usize) as isize)),
        );
    }
}
fn click(window: &SurfaceWindow) {
    mouse(window, WM_LBUTTONDOWN, 10, 9);
    mouse(window, WM_LBUTTONUP, 10, 9);
}
fn key(window: &SurfaceWindow, message: u32, key: VIRTUAL_KEY, repeat: bool) {
    unsafe {
        SendMessageW(
            window.hwnd,
            message,
            Some(WPARAM(key.0 as usize)),
            Some(LPARAM(if repeat { 1 << 30 } else { 0 })),
        );
    }
}
fn event(window: &SurfaceWindow, expected: bool) -> Vec<crate::surface_click_event::ClickEvent> {
    let events = window.prepare_edit(&binding(), 1).unwrap();
    assert_eq!(events.len(), 1);
    let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
    assert_eq!(wire["event"], "change");
    assert_eq!(wire["class"], "commit");
    assert_eq!(wire["payload"]["value"], expected);
    events
}
#[test]
fn surface_switch_window_local_mouse_toggle_survives_submission_pause() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    window.set_submission_enabled(false).unwrap();
    click(&window);
    assert!(value(&window));
    assert!(!window.has_pending_click());
    assert!(window.has_pending_edit());
    assert!(window.prepare_edit(&binding(), 1).is_err());
    window.set_submission_enabled(true).unwrap();
    let events = event(&window, true);
    window.replace(scene(true, false, "change")).unwrap();
    window.settle_edits(&events).unwrap();
    assert!(value(&window));
    unsafe {
        SendMessageW(window.hwnd, WM_PAINT, None, None);
    }
    assert!(window.take_error().is_none());
}
#[test]
fn surface_switch_window_space_release_enter_repeat_and_focus_cancel() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    unsafe {
        SetFocus(Some(window.hwnd)).unwrap();
    }
    key(&window, WM_KEYDOWN, VK_TAB, false);
    assert_eq!(window.state.keyboard.borrow().focused(), Some("switch"));
    key(&window, WM_KEYDOWN, VK_RETURN, false);
    assert!(!value(&window));
    key(&window, WM_KEYDOWN, VK_SPACE, false);
    key(&window, WM_KEYDOWN, VK_SPACE, true);
    assert!(!value(&window));
    key(&window, WM_KEYUP, VK_SPACE, false);
    assert!(value(&window));
    key(&window, WM_KEYUP, VK_SPACE, false);
    assert!(value(&window));
    key(&window, WM_KEYDOWN, VK_SPACE, false);
    unsafe {
        SendMessageW(window.hwnd, WM_CANCELMODE, None, None);
    }
    key(&window, WM_KEYUP, VK_SPACE, false);
    assert!(value(&window));
    assert!(!window.has_pending_click());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_switch_window_fifo_cancel_and_newer_local_value_protection() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    click(&window);
    click(&window);
    assert!(!value(&window));
    let first = event(&window, true);
    window.replace(scene(true, false, "change")).unwrap();
    window.settle_edits(&first).unwrap();
    assert!(!value(&window));
    let second = event(&window, false);
    window.cancel_edits(&second);
    assert!(!value(&window));
    let retry = event(&window, false);
    assert_ne!(second[0].id(), retry[0].id());
    window.settle_edits(&second).unwrap();
    assert!(!value(&window));
    window.settle_edits(&retry).unwrap();
    assert!(value(&window));
}
#[test]
fn surface_switch_window_contract_change_and_disabled_reject_stale_receipts() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    click(&window);
    let old = event(&window, true);
    window.replace(scene(false, false, "new-action")).unwrap();
    assert!(!value(&window));
    click(&window);
    window.settle_edits(&old).unwrap();
    assert!(value(&window));
    let current = event(&window, true);
    let wire: serde_json::Value = serde_json::from_slice(current[0].bytes()).unwrap();
    assert_eq!(wire["action"], "new-action");
    window.replace(scene(false, true, "new-action")).unwrap();
    click(&window);
    assert!(!value(&window));
    assert!(!window.has_pending_edit());
    assert!(!window.has_pending_click());
}

#[test]
fn surface_switch_window_contract_replacement_drops_waiting_and_prepared_identity() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for receipt in ["waiting", "cancel", "fail"] {
        let window = mount();
        click(&window);
        let old = if receipt == "waiting" {
            Vec::new()
        } else {
            event(&window, true)
        };
        window.replace(scene(false, false, "new-action")).unwrap();
        assert!(!window.has_pending_edit(), "{receipt}");
        assert!(window.prepare_edit(&binding(), 2).unwrap().is_empty());
        click(&window);
        if receipt == "cancel" {
            window.cancel_edits(&old);
        }
        if receipt == "fail" {
            window.fail_edits(&old);
        }
        let current = event(&window, true);
        let wire: serde_json::Value = serde_json::from_slice(current[0].bytes()).unwrap();
        assert_eq!(wire["action"], "new-action");
        assert!(!window.has_pending_edit());
    }
}

#[test]
fn surface_switch_window_failure_cuts_dependencies_without_rolling_back_local_state() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    click(&window);
    let first = event(&window, true);
    click(&window);
    window.fail_edits(&first);
    assert!(!value(&window));
    assert!(!window.has_pending_edit());
    window.replace(scene(true, false, "change")).unwrap();
    assert!(!value(&window));
    click(&window);
    let next = event(&window, true);
    window.settle_edits(&next).unwrap();
    assert!(value(&window));
}

#[test]
fn surface_switch_window_shares_text_fifo_without_reordering_or_coalescing_changes() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"switch","type":"switch","events":{"change":"toggle"}},{"id":"text","type":"input","events":{"input":"typing"}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    let edit =
        unsafe { FindWindowExW(Some(window.hwnd), None, windows::core::w!("EDIT"), None).unwrap() };
    let replace = |value: &str| unsafe {
        use windows::Win32::UI::Controls::{EM_REPLACESEL, EM_SETSEL};
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let text = windows::core::HSTRING::from(value);
        SendMessageW(
            edit,
            EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(text.as_ptr() as isize)),
        );
    };
    replace("before");
    click(&window);
    replace("after");
    for (phase, expected) in [
        ("input", serde_json::json!("before")),
        ("change", serde_json::json!(true)),
        ("input", serde_json::json!("after")),
    ] {
        let events = window.prepare_edit(&binding(), 1).unwrap();
        assert_eq!(events.len(), 1);
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["event"], phase);
        assert_eq!(wire["payload"]["value"], expected);
        window.settle_edits(&events).unwrap();
    }
    assert!(!window.has_pending_edit());
}

#[test]
fn surface_switch_window_without_handler_retains_local_until_authoritative_value_changes() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let source = |value| {
        Scene::parse(
            serde_json::json!({"id":"switch","type":"switch","props":{"value":value}})
                .to_string()
                .as_bytes(),
        )
        .unwrap()
    };
    let window = SurfaceWindow::new(source(false), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    click(&window);
    assert!(window.prepare_edit(&binding(), 1).unwrap().is_empty());
    window.replace(source(false)).unwrap();
    assert!(value(&window));
    window.replace(source(true)).unwrap();
    window.replace(source(false)).unwrap();
    assert!(!value(&window));
}
#[test]
fn surface_switch_window_capacity_rejects_before_mutation_and_revocation_cancels_press() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    for _ in 0..32 {
        click(&window);
    }
    assert!(!value(&window));
    click(&window);
    assert!(!value(&window));
    assert!(window.take_error().is_some());
    for index in 0..32 {
        let events = event(&window, index % 2 == 0);
        window.settle_edits(&events).unwrap();
    }
    mouse(&window, WM_LBUTTONDOWN, 10, 9);
    window.set_input_policy(false, false).unwrap();
    mouse(&window, WM_LBUTTONUP, 10, 9);
    assert!(!value(&window));
    assert!(!window.has_pending_edit());
    window.set_input_policy(true, true).unwrap();
    mouse(&window, WM_LBUTTONDOWN, 10, 9);
    mouse(&window, WM_MOUSEMOVE, 200, 90);
    mouse(&window, WM_LBUTTONUP, 10, 9);
    assert!(!value(&window));
}
