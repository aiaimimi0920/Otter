//! 真正 Surface 挂载→子 HWND→父通知→共享 FIFO→wire；不直接调用 notify 绕过消息链。
use super::*;
#[path = "surface_slider_bubble_tests.rs"]
mod bubble;
use crate::{
    desktop::surface_window::SurfaceWindow, surface_live_binding::Binding, surface_scene::Scene,
};
fn binding() -> Binding {
    Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":1},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":1}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
fn scene(value: f64, action: &str) -> Scene {
    Scene::parse(&serde_json::to_vec(&serde_json::json!({"id":"s","type":"slider","props":{"value":value},"events":{"input":"move","change":action}})).unwrap()).unwrap()
}
fn slider(window: &SurfaceWindow) -> HWND {
    unsafe {
        FindWindowExW(
            Some(window.dialog_owner().unwrap()),
            None,
            w!("STATIC"),
            None,
        )
        .unwrap()
    }
}
fn mounted() -> (SurfaceWindow, Binding, HWND) {
    let window = SurfaceWindow::new(scene(50., "commit"), 240, 200).unwrap();
    let binding = binding();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let hwnd = slider(&window);
    unsafe {
        let _ = SetFocus(Some(hwnd));
    }
    (window, binding, hwnd)
}
fn wire(window: &SurfaceWindow, binding: &Binding) -> serde_json::Value {
    let events = window.prepare_edit(binding, 1).unwrap();
    assert_eq!(events.len(), 1);
    let value = serde_json::from_slice(events[0].bytes()).unwrap();
    window.settle_edits(&events).unwrap();
    value
}
#[test]
fn surface_slider_mount_real_keyboard_emits_numeric_input_and_commit_with_retry() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, hwnd) = mounted();
    unsafe {
        key(hwnd, VK_RIGHT);
    }
    assert!(window.has_pending_edit());
    let event = window.prepare_edit(&binding, 1).unwrap();
    assert_eq!(event.len(), 1);
    let original: serde_json::Value = serde_json::from_slice(event[0].bytes()).unwrap();
    assert_eq!(original["class"], "continuous");
    assert_eq!(original["payload"], serde_json::json!({"value":51.}));
    window.cancel_edits(&event);
    let retried = wire(&window, &binding);
    assert_eq!(retried["payload"], original["payload"]);
    let change = wire(&window, &binding);
    assert_eq!(change["event"], "change");
    assert_eq!(change["class"], "commit");
    assert_eq!(change["payload"], serde_json::json!({"value":51.}));
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_slider_mount_drag_coalesces_input_and_preserves_capture_on_identical_sync() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, hwnd) = mounted();
    unsafe {
        let (left, right, _, _) = paint::track(hwnd).unwrap();
        mouse(hwnd, WM_LBUTTONDOWN, left);
        assert_eq!(GetCapture(), hwnd);
        window.replace(scene(50., "commit")).unwrap();
        assert_eq!(slider(&window), hwnd);
        assert_eq!(GetCapture(), hwnd);
        mouse(hwnd, WM_MOUSEMOVE, right);
        mouse(hwnd, WM_LBUTTONUP, right);
    }
    let input = wire(&window, &binding);
    assert_eq!(input["event"], "input");
    assert_eq!(input["payload"], serde_json::json!({"value":100.}));
    let change = wire(&window, &binding);
    assert_eq!(change["event"], "change");
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_slider_mount_queue_full_reverts_next_key_without_half_pair() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, hwnd) = mounted();
    unsafe {
        for _ in 0..16 {
            key(hwnd, VK_RIGHT);
        }
        assert_eq!((*state(hwnd)).value, 66.);
        key(hwnd, VK_RIGHT);
        assert_eq!((*state(hwnd)).value, 66.);
    }
    assert!(window.take_error().is_some());
    for n in 0..32 {
        let value = wire(&window, &binding);
        assert_eq!(
            value["class"],
            if n % 2 == 0 { "continuous" } else { "commit" }
        );
    }
    assert!(!window.has_pending_edit());
}
#[test]
fn surface_slider_mount_authority_contract_and_policy_revoke_old_intents() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    window.replace(scene(20., "commit")).unwrap();
    assert_eq!(slider(&window), hwnd);
    unsafe {
        assert_eq!((*state(hwnd)).value, 20.);
        key(hwnd, VK_RIGHT);
    }
    assert!(window.has_pending_edit());
    window.replace(scene(30., "new-action")).unwrap();
    assert!(!window.has_pending_edit());
    let next = slider(&window);
    unsafe {
        let _ = SetFocus(Some(next));
        key(next, VK_RIGHT);
    }
    assert!(window.has_pending_edit());
    window.set_input_policy(false, false).unwrap();
    assert!(!window.has_pending_edit());
    unsafe {
        assert!(!IsWindowEnabled(next).as_bool());
    }
    assert!(window.take_error().is_none());
}

#[test]
fn surface_slider_mount_tab_routes_to_edit_and_back_without_change() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene=Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"s","type":"slider"},{"id":"disabled","type":"slider","props":{"disabled":true}},{"id":"text","type":"input"}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 200).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let first = slider(&window);
    unsafe {
        let edit =
            FindWindowExW(Some(window.dialog_owner().unwrap()), None, w!("EDIT"), None).unwrap();
        let _ = SetFocus(Some(first));
        key(first, VK_TAB);
        assert_eq!(GetFocus(), edit);
        key(edit, VK_TAB);
        assert_eq!(GetFocus(), first);
    }
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_slider_mount_tab_finishes_drag_in_shared_queue_before_focus_moves() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene=Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"s","type":"slider","props":{"value":50},"events":{"input":"move","change":"commit"}},{"id":"text","type":"input"}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 200).unwrap();
    let binding = binding();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let hwnd = slider(&window);
    unsafe {
        mouse(hwnd, WM_LBUTTONDOWN, 0);
        key(hwnd, VK_TAB);
        let edit =
            FindWindowExW(Some(window.dialog_owner().unwrap()), None, w!("EDIT"), None).unwrap();
        assert_eq!(GetFocus(), edit);
        assert_eq!(GetCapture(), hwnd);
        mouse(hwnd, WM_LBUTTONUP, 0);
        assert_ne!(GetCapture(), hwnd);
    }
    let input = wire(&window, &binding);
    let change = wire(&window, &binding);
    assert_eq!(input["event"], "input");
    assert_eq!(change["event"], "change");
    assert_eq!(input["payload"], serde_json::json!({"value":0.0}));
    assert_eq!(input["payload"], change["payload"]);
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}

#[test]
fn surface_slider_mount_parent_cancel_releases_drag_and_kind_change_drops_old_intent() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, hwnd) = mounted();
    unsafe {
        mouse(hwnd, WM_LBUTTONDOWN, 0);
        assert_eq!(GetCapture(), hwnd);
        SendMessageW(window.dialog_owner().unwrap(), WM_CANCELMODE, None, None);
        assert_ne!(GetCapture(), hwnd);
    }
    assert!(window.has_pending_edit());
    window.replace(Scene::parse(br#"{"id":"s","type":"select","props":{"value":"a","options":[{"value":"a","label":"A"},{"value":"b","label":"B"}]},"events":{"change":"commit"}}"#).unwrap()).unwrap();
    assert!(!window.has_pending_edit());
    unsafe {
        assert!(!IsWindow(Some(hwnd)).as_bool());
        let select = FindWindowExW(
            Some(window.dialog_owner().unwrap()),
            None,
            w!("COMBOBOX"),
            None,
        )
        .unwrap();
        key(select, VK_DOWN);
    }
    assert!(window.has_pending_edit());
    window.replace(scene(50., "commit")).unwrap();
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
