//! Switch 单次激活的祖先 click 先于 change；不复制 DOM label 转发的重复 click。
use super::*;
use crate::desktop::surface_edit_mount::FenceState;
fn nested() -> SurfaceWindow {
    nested_with_handler(true)
}
fn nested_with_handler(handler: bool) -> SurfaceWindow {
    let scene = Scene::parse(br#"{"id":"outer","type":"column","events":{"click":"outer-action"},"children":[{"id":"inner","type":"column","events":{"click":"inner-action"},"children":[{"id":"switch","type":"switch","events":{"change":"toggle"}}]},{"id":"text","type":"input","events":{"input":"typing"}}]}"#).unwrap();
    let scene = if handler {
        scene
    } else {
        Scene::parse(br#"{"id":"outer","type":"column","events":{"click":"outer-action"},"children":[{"id":"inner","type":"column","events":{"click":"inner-action"},"children":[{"id":"switch","type":"switch","events":{"change":"toggle"}}]},{"id":"text","type":"input"}]}"#).unwrap()
    };
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window
}
fn ancestors(window: &SurfaceWindow, pending: &DeferredClick) {
    let events = window.prepare_deferred(pending, &binding(), 1).unwrap();
    let wires: Vec<serde_json::Value> = events
        .iter()
        .map(|event| serde_json::from_slice(event.bytes()).unwrap())
        .collect();
    assert_eq!(wires.len(), 2);
    assert_eq!(wires[0]["nodeId"], "inner");
    assert_eq!(wires[1]["nodeId"], "outer");
    assert!(wires
        .iter()
        .all(|wire| wire["event"] == "click" && wire["class"] == "discrete"));
}
#[test]
fn surface_switch_bubble_mouse_orders_one_ancestor_batch_before_change() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = nested();
    click(&window);
    assert!(value(&window));
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding(), 1)
        .unwrap();
    // 自身 change 仍在 waiting，但不属于祖先 click 的前置依赖。
    assert_eq!(window.deferred_state(&pending), FenceState::Ready);
    assert!(window.has_pending_edit());
    ancestors(&window, &pending);
    event(&window, true);
    assert!(window.take_click().is_none());
}
#[test]
fn surface_switch_bubble_waits_for_earlier_text_but_not_its_own_change() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = nested();
    let edit =
        unsafe { FindWindowExW(Some(window.hwnd), None, windows::core::w!("EDIT"), None).unwrap() };
    unsafe {
        let text = windows::core::HSTRING::from("before");
        SendMessageW(
            edit,
            windows::Win32::UI::Controls::EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(text.as_ptr() as isize)),
        );
    }
    click(&window);
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding(), 1)
        .unwrap();
    assert_eq!(window.deferred_state(&pending), FenceState::Pending);
    assert!(window.prepare_deferred(&pending, &binding(), 1).is_err());
    let earlier = window.prepare_edit(&binding(), 1).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(earlier[0].bytes()).unwrap();
    assert_eq!(wire["payload"]["value"], "before");
    assert_eq!(window.deferred_state(&pending), FenceState::InFlight);
    window.settle_edits(&earlier).unwrap();
    assert_eq!(window.deferred_state(&pending), FenceState::Ready);
    ancestors(&window, &pending);
    event(&window, true);
}

#[test]
fn surface_switch_bubble_handlerless_prefix_cannot_prepare_change_across_fence() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = nested_with_handler(false);
    let edit =
        unsafe { FindWindowExW(Some(window.hwnd), None, windows::core::w!("EDIT"), None).unwrap() };
    unsafe {
        let text = windows::core::HSTRING::from("local only");
        SendMessageW(
            edit,
            windows::Win32::UI::Controls::EM_REPLACESEL,
            Some(WPARAM(1)),
            Some(LPARAM(text.as_ptr() as isize)),
        );
    }
    click(&window);
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding(), 1)
        .unwrap();
    assert_eq!(window.deferred_state(&pending), FenceState::Pending);
    let prefix = window
        .prepare_edit_before(&binding(), 1, Some(&pending))
        .unwrap();
    assert!(prefix.is_empty());
    assert_eq!(window.deferred_state(&pending), FenceState::Ready);
    assert!(window.has_pending_edit());
    ancestors(&window, &pending);
    event(&window, true);
}
#[test]
fn surface_switch_bubble_space_and_soft_submission_stop_keep_distinct_gates() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = nested();
    unsafe {
        SetFocus(Some(window.hwnd)).unwrap();
    }
    key(&window, WM_KEYDOWN, VK_TAB, false);
    key(&window, WM_KEYDOWN, VK_SPACE, false);
    key(&window, WM_KEYUP, VK_SPACE, false);
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding(), 1)
        .unwrap();
    ancestors(&window, &pending);
    let events = event(&window, true);
    window.settle_edits(&events).unwrap();
    window.set_submission_enabled(false).unwrap();
    click(&window);
    assert!(value(&window));
    assert!(window.take_click().is_none());
    window.set_submission_enabled(true).unwrap();
    assert!(window.take_click().is_none());
    event(&window, true);
}
#[test]
fn surface_switch_bubble_disabled_and_replaced_contract_do_not_emit_old_actions() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = nested();
    click(&window);
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding(), 1)
        .unwrap();
    window.replace(Scene::parse(br#"{"id":"outer","type":"column","events":{"click":"new-action"},"children":[{"id":"switch","type":"switch","props":{"disabled":true},"events":{"change":"toggle"}}]}"#).unwrap()).unwrap();
    assert!(window.prepare_deferred(&pending, &binding(), 2).is_err());
    click(&window);
    assert!(window.take_click().is_none());
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
