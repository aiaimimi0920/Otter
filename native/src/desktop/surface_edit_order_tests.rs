//! 真实编辑通知保留 change 边沿及跨字段 FIFO，不让后续 input 覆盖较早提交值。
use super::*;
#[test]
fn surface_edit_deadline_preserves_text_but_never_replays_expired_intent_on_blur() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let edit = child(&window);
    type_text(edit, "expired local");
    let future = std::time::Instant::now() + std::time::Duration::from_secs(31);
    window
        .state
        .run(|host| {
            assert!(host.prepare_edit_at(&binding(), 1, future).is_err());
            Ok(())
        })
        .unwrap();
    assert_eq!(text(edit), "expired local");
    assert!(!window.has_pending_edit());
    commit(&window, edit);
    assert!(!window.has_pending_edit());
    type_text(edit, "explicit fresh edit");
    assert_eq!(next(&window).1["payload"]["value"], "explicit fresh edit");
}
#[test]
fn surface_edit_order_full_queue_reports_failure_without_stale_form_values() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let edit = child(&window);
    for n in 0..16 {
        type_text(edit, &n.to_string());
        commit(&window, edit);
    }
    assert!(window.take_error().is_none());
    type_text(edit, "overflow local");
    assert!(window.take_error().is_some());
    assert_eq!(text(edit), "overflow local");
    window
        .state
        .run(|host| {
            assert_eq!(host.capture_form_values()["field"], "overflow local");
            Ok(())
        })
        .unwrap();
    assert_eq!(next(&window).1["payload"]["value"], "0");
}
fn commit(window: &SurfaceWindow, edit: HWND) {
    unsafe {
        SendMessageW(
            window.hwnd,
            WM_COMMAND,
            Some(WPARAM((EN_KILLFOCUS as usize) << 16)),
            Some(LPARAM(edit.0 as isize)),
        );
    }
}
fn next(
    window: &SurfaceWindow,
) -> (
    Vec<crate::surface_click_event::ClickEvent>,
    serde_json::Value,
) {
    let events = window.prepare_edit(&binding(), 1).unwrap();
    assert_eq!(events.len(), 1);
    let wire = serde_json::from_slice(events[0].bytes()).unwrap();
    (events, wire)
}
#[test]
fn surface_edit_order_change_is_a_coalescing_boundary_and_cancellation_keeps_value() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    let edit = child(&window);
    type_text(edit, "A");
    type_text(edit, "B");
    commit(&window, edit);
    type_text(edit, "C");
    for (phase, value) in [("input", "B"), ("change", "B"), ("input", "C")] {
        let (events, wire) = next(&window);
        assert_eq!(wire["event"], phase);
        assert_eq!(wire["payload"]["value"], value);
        window.cancel_edits(&events);
        let (retry, wire) = next(&window);
        assert_eq!(wire["event"], phase);
        assert_eq!(wire["payload"]["value"], value);
        window.settle_edits(&retry).unwrap();
        if value != "C" {
            assert_eq!(text(edit), "C");
        }
    }
    assert!(!window.has_pending_edit());
}
#[test]
fn surface_edit_order_cross_field_notifications_are_not_sorted_by_node_id() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"z","type":"input","events":{"input":"z"}},{"id":"a","type":"input","events":{"input":"a"}}]}"#).unwrap();
    let window = SurfaceWindow::new(scene, 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    // 子窗口创建顺序与场景一致，不依赖原生 z-order。
    let mut fields = Vec::new();
    window
        .state
        .run(|host| {
            host.focus_edit("z")?;
            fields.push(unsafe { GetFocus() });
            host.focus_edit("a")?;
            fields.push(unsafe { GetFocus() });
            Ok(())
        })
        .unwrap();
    type_text(fields[0], "first");
    type_text(fields[1], "second");
    assert_eq!(next(&window).1["nodeId"], "z");
    assert_eq!(next(&window).1["nodeId"], "a");
}
