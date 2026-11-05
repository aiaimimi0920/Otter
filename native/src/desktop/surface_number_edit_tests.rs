//! 真实 EDIT 的 Number 字符串草稿、数值事件、步进与撤权路径；不注入桌面输入。
use super::*;
#[path = "surface_number_erase_tests.rs"]
mod erase;
#[path = "surface_number_filter_tests.rs"]
mod filter;
#[path = "surface_number_paste_tests.rs"]
mod paste;
#[path = "surface_number_spinner_tests.rs"]
mod spinner;
#[test]
fn surface_number_placeholder_contract_never_creates_a_draft_or_event() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let scene = |hint: &str| {
        Scene::parse(serde_json::json!({"id":"field","type":"number","props":{"value":"","placeholder":hint,"min":0,"max":10},"layout":{"height":32},"events":{"input":"typing","change":"commit"}}).to_string().as_bytes()).unwrap()
    };
    let window = SurfaceWindow::new(scene("请输入数量"), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    assert_eq!(text(child(&window)), "");
    assert_eq!(values(&window)["field"], "");
    assert!(!window.has_pending_edit());
    window.replace(scene("新的提示")).unwrap();
    assert_eq!(text(child(&window)), "");
    assert!(!window.has_pending_edit());
    type_text(child(&window), "3.5");
    let (_, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 3.5);
    assert_eq!(values(&window)["field"], "3.5");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_edit_full_queue_rejects_whole_step_before_visible_mutation() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    for index in 0..31 {
        type_text(edit, &format!("0.{index}"));
        values(&window); // 固定边沿，禁止下一次 input 合并旧值。
    }
    unsafe {
        SetFocus(Some(edit)).unwrap();
    }
    arrow(edit, true);
    assert_eq!(text(edit), "0.30");
    assert!(window.take_error().is_some());
    let mut seen = 0;
    while window.has_pending_edit() {
        let (events, wire) = event(&window);
        assert_eq!(wire["event"], "input");
        window.settle_edits(&events).unwrap();
        seen += 1;
    }
    assert_eq!(seen, 31);
    arrow(edit, true);
    let (_, input) = event(&window);
    let (_, change) = event(&window);
    assert_eq!(input["event"], "input");
    assert_eq!(change["event"], "change");
    assert_eq!(input["payload"], change["payload"]);
    assert!(window.take_error().is_none());
}
fn form(value: serde_json::Value, step: f64) -> Scene {
    Scene::parse(serde_json::json!({"id":"field","type":"number","props":{"value":value,"min":-10,"max":10,"step":step},"layout":{"height":32},"events":{"input":"typing","change":"commit"}}).to_string().as_bytes()).unwrap()
}
fn mount(value: serde_json::Value) -> SurfaceWindow {
    let window = SurfaceWindow::new(form(value, 0.5), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window
}
fn event(
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
fn values(window: &SurfaceWindow) -> serde_json::Value {
    let mut values = Default::default();
    window
        .state
        .run(|host| {
            values = host.capture_form_values();
            Ok(())
        })
        .unwrap();
    serde_json::Value::Object(values)
}
fn arrow(edit: HWND, up: bool) {
    unsafe {
        SendMessageW(
            edit,
            WM_KEYDOWN,
            Some(WPARAM(if up { VK_UP.0 } else { VK_DOWN.0 } as usize)),
            None,
        );
    }
}
#[test]
fn surface_number_edit_preserves_string_form_and_sends_numeric_or_null_values() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1.00"));
    let edit = child(&window);
    assert_eq!(text(edit), "1.00");
    assert_eq!(values(&window)["field"], "1.00");
    for (raw, expected, form) in [
        ("2.50", serde_json::json!(2.5), "2.50"),
        ("-", serde_json::Value::Null, ""),
        ("1e2", serde_json::json!(100.), "1e2"),
        ("", serde_json::Value::Null, ""),
    ] {
        type_text(edit, raw);
        assert_eq!(text(edit), raw);
        assert_eq!(values(&window)["field"], form);
        let (_, wire) = event(&window);
        assert_eq!(wire["payload"]["value"], expected);
    }
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_edit_sanitizes_authority_but_does_not_clamp_typed_values() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for value in [
        serde_json::json!(12),
        serde_json::json!("NaN"),
        serde_json::json!("1e400"),
        serde_json::json!("1."),
    ] {
        let window = mount(value);
        assert_eq!(text(child(&window)), "");
        assert!(!window.has_pending_edit());
    }
    let window = mount(serde_json::json!("1"));
    type_text(child(&window), "200.25");
    let (_, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 200.25);
}
#[test]
fn surface_number_edit_pending_ack_keeps_later_raw_draft_and_snapshot() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    type_text(edit, "2.00");
    let (first, wire) = event(&window);
    assert_eq!(wire["payload"]["value"], 2.);
    type_text(edit, "3e");
    window.replace(form(serde_json::json!("4"), 0.5)).unwrap();
    window.settle_edits(&first).unwrap();
    assert_eq!(text(edit), "3e");
    assert_eq!(values(&window)["field"], "");
    let (second, wire) = event(&window);
    assert!(wire["payload"]["value"].is_null());
    window.settle_edits(&second).unwrap();
    assert_eq!(text(edit), "4");
    assert_eq!(values(&window)["field"], "4");
}
#[test]
fn surface_number_edit_keyboard_step_emits_ordered_input_and_change_without_submission_rights() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1.2"));
    let edit = child(&window);
    unsafe {
        SetFocus(Some(edit)).unwrap();
    }
    window.set_submission_enabled(false).unwrap();
    arrow(edit, true);
    assert_eq!(text(edit), "1.5");
    assert_eq!(values(&window)["field"], "1.5");
    assert!(window.prepare_edit(&binding(), 1).is_err());
    window.set_submission_enabled(true).unwrap();
    for phase in ["input", "change"] {
        let (_, wire) = event(&window);
        assert_eq!(wire["event"], phase);
        assert_eq!(wire["payload"]["value"], 1.5);
    }
    assert!(!window.has_pending_edit());
    window.set_input_policy(false, false).unwrap();
    arrow(edit, true);
    assert_eq!(text(edit), "1.5");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_number_edit_composition_blocks_steps_and_contract_change_discards_old_queue() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount(serde_json::json!("1"));
    let edit = child(&window);
    unsafe {
        SetFocus(Some(edit)).unwrap();
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
    }
    arrow(edit, true);
    assert_eq!(text(edit), "1");
    unsafe {
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
    }
    arrow(edit, true);
    assert_eq!(text(edit), "1.5");
    window.replace(form(serde_json::json!("8"), 1.)).unwrap();
    assert!(!window.has_pending_edit());
    assert_eq!(text(child(&window)), "8");
    let current = child(&window);
    window.close().unwrap();
    assert!(!unsafe { IsWindow(Some(current)) }.as_bool());
}
