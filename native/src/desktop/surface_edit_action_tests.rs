//! 同 ID 控件更换动作不能接收旧合同下的草稿、phase 或回执。
use super::*;
#[test]
fn surface_edit_contract_change_discards_old_intent_even_if_action_is_unchanged() {
    let _com = crate::desktop::Apartment::new().unwrap();
    for section in ["props", "style", "layout", "accessibility"] {
        let window = mount();
        type_text(child(&window), "old intent");
        let mut node = serde_json::json!({"id":"field","type":"input", "props":{"value":"new source"},
            "layout":{"width":180,"height":32},"events":{"input":"typing","change":"commit"}});
        match section {
            "props" => node["props"]["placeholder"] = "changed".into(),
            "style" => node["style"] = serde_json::json!({"color":"#ff0000"}),
            "layout" => node["layout"]["width"] = 160.into(),
            _ => node["accessibility"] = serde_json::json!({"label":"changed"}),
        }
        window
            .replace(Scene::parse(node.to_string().as_bytes()).unwrap())
            .unwrap();
        assert!(!window.has_pending_edit(), "{section}");
        assert_eq!(text(child(&window)), "new source");
        type_text(child(&window), "new intent");
        let events = window.prepare_edit(&binding(), 2).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(events[0].bytes()).unwrap()["payload"]
                ["value"],
            "new intent"
        );
    }
}
fn source(value: &str, input: &str, change: &str) -> Scene {
    Scene::parse(
        serde_json::json!({"id":"field","type":"input","props":{"value":value},
        "events":{"input":input,"change":change}})
        .to_string()
        .as_bytes(),
    )
    .unwrap()
}
#[test]
fn surface_edit_action_change_discards_old_draft_and_queued_phases() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    type_text(child(&window), "old local intent");
    assert!(window.has_pending_edit());
    window
        .replace(source("new source", "new action", "new commit"))
        .unwrap();
    assert!(!window.has_pending_edit());
    assert_eq!(text(child(&window)), "new source");
    assert!(window.prepare_edit(&binding(), 2).unwrap().is_empty());
    type_text(child(&window), "new intent");
    let events = window.prepare_edit(&binding(), 2).unwrap();
    let wire: serde_json::Value = serde_json::from_slice(events[0].bytes()).unwrap();
    assert_eq!(wire["action"], "new action");
    assert_eq!(wire["payload"]["value"], "new intent");
    assert!(window.take_error().is_none());
}
#[test]
fn surface_edit_action_change_late_old_receipt_cannot_settle_new_session() {
    let _com = crate::desktop::Apartment::new().unwrap();
    let window = mount();
    type_text(child(&window), "old");
    let old = window.prepare_edit(&binding(), 1).unwrap();
    window
        .replace(source("new source", "typing", "different commit"))
        .unwrap();
    type_text(child(&window), "new pending");
    let new = window.prepare_edit(&binding(), 2).unwrap();
    window.fail_edits(&old);
    window.settle_edits(&old).unwrap();
    assert_eq!(text(child(&window)), "new pending");
    window
        .replace(source("accepted", "typing", "different commit"))
        .unwrap();
    window.settle_edits(&new).unwrap();
    assert_eq!(text(child(&window)), "accepted");
    assert!(window.take_error().is_none());
}
