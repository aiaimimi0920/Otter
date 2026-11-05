//! 子 Slider 点击在自身 input/change 后冒泡，不复用 Switch 的 click-before-change 顺序。
use super::*;
use crate::desktop::surface_edit_mount::FenceState;
#[test]
fn surface_slider_bubble_tab_then_up_click_does_not_duplicate_change() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, child) = mount();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, right(child));
        key(child, VK_TAB);
        mouse(child, WM_LBUTTONUP, right(child));
    }
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding, 1)
        .unwrap();
    for expected in ["input", "change"] {
        let events = window
            .prepare_edit_before(&binding, 1, Some(&pending))
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["event"],
            expected
        );
        window.settle_edits(&events).unwrap();
    }
    assert!(!window.has_pending_edit());
    assert_eq!(
        window
            .prepare_deferred(&pending, &binding, 1)
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn surface_slider_bubble_tab_then_cancel_revokes_capture_and_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, child) = mount();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, right(child));
        key(child, VK_TAB);
        assert_eq!(GetCapture(), child);
        SendMessageW(window.dialog_owner().unwrap(), WM_CANCELMODE, None, None);
        assert_ne!(GetCapture(), child);
        mouse(child, WM_LBUTTONUP, 1);
    }
    assert!(window.take_click().is_none());
    assert_eq!(wire(&window, &binding)["event"], "input");
    assert_eq!(wire(&window, &binding)["event"], "change");
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_slider_bubble_tab_or_focus_change_preserves_drag_and_original_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for tab in [true, false] {
        let (window, binding, child) = mount();
        unsafe {
            let edit = FindWindowExW(Some(window.dialog_owner().unwrap()), None, w!("EDIT"), None)
                .unwrap();
            mouse(child, WM_LBUTTONDOWN, right(child));
            if tab {
                key(child, VK_TAB);
            } else {
                let _ = SetFocus(Some(edit));
            }
            assert_eq!(GetFocus(), edit);
            assert_eq!(GetCapture(), child);
            mouse(child, WM_MOUSEMOVE, 1);
            mouse(child, WM_LBUTTONUP, 1);
            assert_ne!(GetCapture(), child);
        }
        let pending = window
            .defer_click(
                window.take_click().expect("click lost across focus change"),
                &binding,
                1,
            )
            .unwrap();
        for (event, value) in [
            ("input", 100.0),
            ("change", 100.0),
            ("input", 0.0),
            ("change", 0.0),
        ] {
            let events = window
                .prepare_edit_before(&binding, 1, Some(&pending))
                .unwrap();
            assert_eq!(events.len(), 1);
            let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
            assert_eq!(wire["event"], event);
            assert_eq!(wire["payload"]["value"], value);
            window.settle_edits(&events).unwrap();
        }
        assert_eq!(
            window
                .prepare_deferred(&pending, &binding, 1)
                .unwrap()
                .len(),
            2
        );
        assert!(window.take_error().is_none());
    }
}
#[test]
fn surface_slider_bubble_release_over_sibling_does_not_retarget_to_sibling() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, child) = mount();
    unsafe {
        let edit =
            FindWindowExW(Some(window.dialog_owner().unwrap()), None, w!("EDIT"), None).unwrap();
        let mut point = POINT { x: 1, y: 15 };
        MapWindowPoints(Some(edit), Some(child), std::slice::from_mut(&mut point));
        mouse(child, WM_LBUTTONDOWN, 1);
        SendMessageW(
            child,
            WM_LBUTTONUP,
            None,
            Some(LPARAM(
                ((point.x as u16 as u32) | ((point.y as u16 as u32) << 16)) as isize,
            )),
        );
    }
    let click = window.take_click().unwrap();
    assert_eq!(click.node_id, "s");
    let pending = window.defer_click(click, &binding, 1).unwrap();
    assert_eq!(
        window
            .prepare_deferred(&pending, &binding, 1)
            .unwrap()
            .len(),
        2
    );
    assert!(!window.has_pending_edit());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_slider_bubble_captured_release_outside_retains_slider_route() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for x in [-20, 500] {
        let (window, binding, child) = mount();
        unsafe {
            mouse(child, WM_LBUTTONDOWN, 1);
            mouse(child, WM_MOUSEMOVE, x);
            mouse(child, WM_LBUTTONUP, x);
        }
        let pending = window
            .defer_click(
                window.take_click().expect("captured click missing"),
                &binding,
                1,
            )
            .unwrap();
        for _ in 0..4 {
            if window.deferred_state(&pending) != FenceState::Pending {
                break;
            }
            let events = window
                .prepare_edit_before(&binding, 1, Some(&pending))
                .unwrap();
            window.settle_edits(&events).unwrap();
        }
        assert_eq!(window.deferred_state(&pending), FenceState::Ready);
        let events = window.prepare_deferred(&pending, &binding, 1).unwrap();
        let ids: Vec<String> = events
            .iter()
            .map(|event| {
                serde_json::from_slice::<serde_json::Value>(event.bytes()).unwrap()["nodeId"]
                    .as_str()
                    .unwrap()
                    .into()
            })
            .collect();
        assert_eq!(ids, ["inner", "outer"]);
        assert!(window.take_error().is_none());
    }
}
#[test]
fn surface_slider_bubble_captured_click_cannot_survive_revoked_scene() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, child) = mount();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
        mouse(child, WM_LBUTTONUP, -20);
    }
    let click = window.take_click().unwrap();
    window.replace(scene()).unwrap();
    assert!(window.defer_click(click, &binding, 1).is_err());
}
#[test]
fn surface_slider_bubble_outside_begin_cannot_forge_capture_origin() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, child) = mount();
    unsafe {
        SetCapture(child);
        let parent = window.dialog_owner().unwrap();
        SendMessageW(
            parent,
            CLICK_BEGIN,
            Some(WPARAM((-20i16 as u16 as usize) | (15 << 16))),
            Some(LPARAM(child.0 as isize)),
        );
        let _ = ReleaseCapture();
        SendMessageW(
            parent,
            CLICK_END,
            Some(WPARAM(1 | (15 << 16))),
            Some(LPARAM(child.0 as isize)),
        );
    }
    assert!(window.take_click().is_none());
}
#[test]
fn surface_slider_bubble_handlerless_control_does_not_occupy_click_slot() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, child) = mounted();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
        mouse(child, WM_LBUTTONUP, 1);
    }
    assert!(window.has_pending_edit());
    assert!(window.take_click().is_none());
}
#[test]
fn surface_slider_bubble_pending_click_is_not_overwritten_and_disable_cancels_press() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, child) = mount();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
        mouse(child, WM_LBUTTONUP, 1);
        mouse(child, WM_LBUTTONDOWN, 1);
        mouse(child, WM_LBUTTONUP, 1);
    }
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding, 1)
        .unwrap();
    assert_eq!(
        window
            .prepare_deferred(&pending, &binding, 1)
            .unwrap()
            .len(),
        2
    );
    assert!(window.take_click().is_none());
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
    }
    window.set_input_policy(false, false).unwrap();
    unsafe {
        mouse(child, WM_LBUTTONUP, 1);
    }
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
fn scene() -> Scene {
    Scene::parse(br#"{"id":"outer","type":"column","events":{"click":"outer"},"children":[{"id":"inner","type":"column","events":{"click":"inner"},"children":[{"id":"s","type":"slider","props":{"value":0},"events":{"input":"move","change":"commit","click":"ignored"}}]},{"id":"text","type":"input"}]}"#).unwrap()
}
fn mount() -> (SurfaceWindow, Binding, HWND) {
    let window = SurfaceWindow::new(scene(), 300, 240).unwrap();
    let binding = binding();
    window.bind_edits(&binding).unwrap();
    window.set_input_policy(true, true).unwrap();
    window.show().unwrap();
    let child = slider(&window);
    (window, binding, child)
}
unsafe fn right(child: HWND) -> i32 {
    let mut rect = RECT::default();
    GetClientRect(child, &mut rect).unwrap();
    rect.right - 2
}
#[test]
fn surface_slider_bubble_drag_commits_before_inner_outer_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, child) = mount();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
        mouse(child, WM_MOUSEMOVE, right(child));
        mouse(child, WM_LBUTTONUP, right(child));
    }
    let pending = window
        .defer_click(
            window.take_click().expect("Slider click missing"),
            &binding,
            1,
        )
        .unwrap();
    assert_eq!(window.deferred_state(&pending), FenceState::Pending);
    assert!(window.prepare_deferred(&pending, &binding, 1).is_err());
    for expected in ["input", "change"] {
        let events = window
            .prepare_edit_before(&binding, 1, Some(&pending))
            .unwrap();
        assert_eq!(events.len(), 1);
        let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
        assert_eq!(wire["event"], expected);
        assert_eq!(wire["payload"]["value"], 100.0);
        window.settle_edits(&events).unwrap();
    }
    assert_eq!(window.deferred_state(&pending), FenceState::Ready);
    let events = window.prepare_deferred(&pending, &binding, 1).unwrap();
    let wires: Vec<serde_json::Value> = events
        .iter()
        .map(|event| serde_json::from_slice(event.bytes()).unwrap())
        .collect();
    assert_eq!(wires.len(), 2);
    assert_eq!(wires[0]["nodeId"], "inner");
    assert_eq!(wires[1]["nodeId"], "outer");
    assert!(wires.iter().all(|wire| wire["event"] == "click"));
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_slider_bubble_same_value_click_has_no_fake_change_and_duplicate_up_is_ignored() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, binding, child) = mount();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
        mouse(child, WM_LBUTTONUP, 1);
    }
    assert!(!window.has_pending_edit());
    let pending = window
        .defer_click(window.take_click().unwrap(), &binding, 1)
        .unwrap();
    assert_eq!(window.deferred_state(&pending), FenceState::Ready);
    assert_eq!(
        window
            .prepare_deferred(&pending, &binding, 1)
            .unwrap()
            .len(),
        2
    );
    unsafe {
        mouse(child, WM_LBUTTONUP, 1);
    }
    assert!(window.take_click().is_none());
}
#[test]
fn surface_slider_bubble_cancel_and_replacement_do_not_deliver_stale_click() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, child) = mount();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
        SendMessageW(child, WM_CANCELMODE, None, None);
        mouse(child, WM_LBUTTONUP, 1);
    }
    assert!(window.take_click().is_none());
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
    }
    window.replace(scene()).unwrap();
    unsafe {
        mouse(child, WM_LBUTTONUP, 1);
    }
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
#[test]
fn surface_slider_bubble_soft_stop_keeps_local_values_without_click_and_foreign_child_rejected() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let (window, _, child) = mount();
    window.set_input_policy(true, false).unwrap();
    unsafe {
        mouse(child, WM_LBUTTONDOWN, 1);
        mouse(child, WM_MOUSEMOVE, right(child));
        mouse(child, WM_LBUTTONUP, right(child));
    }
    assert!(window.has_pending_edit());
    assert!(window.take_click().is_none());
    window.set_input_policy(true, true).unwrap();
    unsafe {
        let parent = window.dialog_owner().unwrap();
        let edit = FindWindowExW(Some(parent), None, w!("EDIT"), None).unwrap();
        SendMessageW(
            parent,
            CLICK_BEGIN,
            Some(WPARAM(1 | (15 << 16))),
            Some(LPARAM(edit.0 as isize)),
        );
        SendMessageW(
            parent,
            CLICK_END,
            Some(WPARAM(1 | (15 << 16))),
            Some(LPARAM(edit.0 as isize)),
        );
    }
    assert!(window.take_click().is_none());
    assert!(window.take_error().is_none());
}
