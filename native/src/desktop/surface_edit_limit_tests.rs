//! 从真实 EDIT 通知验证 maxlength；程序权威值与用户编辑限制分开。
use super::*;
fn area(value: &str, limit: serde_json::Value) -> Scene {
    Scene::parse(serde_json::json!({"id":"field","type":"textarea","props":{"value":value,"maxLength":limit},"events":{"input":"typing"}}).to_string().as_bytes()).unwrap()
}
fn area_window(value: &str, limit: serde_json::Value) -> SurfaceWindow {
    let window = SurfaceWindow::new(area(value, limit), 240, 160).unwrap();
    window.bind_edits(&binding()).unwrap();
    window.set_input_policy(true, true).unwrap();
    window
}
#[test]
fn surface_edit_limit_counts_utf16_after_newline_normalization() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = area_window("", 4.into());
    let edit = child(&window);
    type_text(edit, "中\r\n🙂");
    assert!(window.take_error().is_none());
    let events = window.prepare_edit(&binding(), 1).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["payload"]["value"],
        "中\n🙂"
    );
    type_text(edit, "中\r\n🙂a");
    assert!(window.take_error().is_some());
    assert_eq!(text(edit), "中\r\n🙂");
    assert!(!window.has_pending_edit());
}
#[test]
fn surface_edit_limit_authoritative_value_is_not_truncated_and_can_be_shortened() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = area_window("abcdef", 2.into());
    let edit = child(&window);
    assert_eq!(text(edit), "abcdef");
    type_text(edit, "abcde");
    assert!(window.take_error().is_none());
    type_text(edit, "abcdef");
    assert!(window.take_error().is_some());
    assert_eq!(text(edit), "abcde");
    type_text(edit, "ab");
    assert!(window.take_error().is_none());
    let events = window.prepare_edit(&binding(), 1).unwrap();
    window.replace(area("server value", 1.into())).unwrap();
    window.settle_edits(&events).unwrap();
    assert_eq!(text(child(&window)), "server value");
}
#[test]
fn surface_edit_limit_rejected_composition_cleans_state_and_accepts_next_edit() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = area_window("ok", 3.into());
    let edit = child(&window);
    unsafe {
        SendMessageW(edit, WM_IME_STARTCOMPOSITION, None, None);
    }
    type_text(edit, "too long");
    assert!(!window.has_pending_edit());
    unsafe {
        SendMessageW(edit, WM_IME_ENDCOMPOSITION, None, None);
    }
    assert!(window.take_error().is_some());
    assert_eq!(text(edit), "ok");
    assert!(!window.has_pending_edit());
    type_text(edit, "yes");
    assert!(window.take_error().is_none());
    assert_eq!(window.prepare_edit(&binding(), 1).unwrap().len(), 1);
}
#[test]
fn surface_edit_limit_clamps_numeric_properties_and_ignores_non_numeric_values() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for (limit, accepted, rejected) in [
        (serde_json::json!(0), "a", "ab"),
        (serde_json::json!(2.8), "ab", "abc"),
    ] {
        let window = area_window("", limit);
        let edit = child(&window);
        type_text(edit, accepted);
        assert!(window.take_error().is_none());
        type_text(edit, rejected);
        assert!(window.take_error().is_some());
        assert_eq!(text(edit), accepted);
    }
    let window = area_window("", serde_json::json!("1"));
    type_text(child(&window), "not numeric");
    assert!(window.take_error().is_none());
}
