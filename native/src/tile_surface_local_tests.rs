//! 独立值oracle；保护同源保留/新权威采用、合同退休和文本共享字段预算。
use super::*;
use crate::tile_surface_value::tests::{fixture, presentation};
use serde_json::json;
struct Apartment;
impl Apartment {
    fn new() -> Self {
        unsafe {
            windows::Win32::System::Com::CoInitializeEx(
                None,
                windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
            )
            .ok()
            .unwrap();
        }
        Self
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}
fn scene(kind: &str, value: serde_json::Value) -> serde_json::Value {
    json!({"id":"field","type":kind,"layout":{"width":64,"height":64},
        "props":{"value":value,"min":0,"max":10,"step":2,"options":[
            {"value":"a","label":"A"},{"value":"b","label":"B"}]}})
}
fn focus(scene: serde_json::Value) -> Focus {
    let f = fixture(scene);
    crate::tile_surface_focus::at(&f.frame, &f.art, &presentation(&f), [0, 0]).unwrap()
}
#[test]
fn same_authoritative_source_retains_local_value_but_changed_source_replaces_it() {
    let _com = Apartment::new();
    for (kind, source_value, edited, next_value, expected) in [
        (
            "switch",
            json!(false),
            Value::Switch(true),
            json!(true),
            Value::Switch(true),
        ),
        (
            "select",
            json!("a"),
            Value::Select(Some(1)),
            json!("b"),
            Value::Select(Some(1)),
        ),
        (
            "slider",
            json!(2),
            Value::Slider(8.),
            json!(4),
            Value::Slider(4.),
        ),
    ] {
        let mut c = Cache::default();
        let f = focus(scene(kind, source_value.clone()));
        c.set(f.clone(), edited, (0, 0)).unwrap();
        let next = focus(scene(kind, source_value));
        assert!(!Arc::ptr_eq(&f.anchor.snapshot, &next.anchor.snapshot));
        assert_eq!(c.value(&next), Some(edited));
        let next = focus(scene(kind, next_value));
        assert_eq!(c.value(&next), Some(expected));
        assert_eq!(c.snapshot().len(), 1);
    }
    // 两个不匹配的声明字符串都投影为None，但新权威字符串仍必须淘汰本地索引。
    let mut c = Cache::default();
    c.set(
        focus(scene("select", json!("missing1"))),
        Value::Select(Some(1)),
        (0, 0),
    )
    .unwrap();
    assert_eq!(
        c.value(&focus(scene("select", json!("missing2")))),
        Some(Value::Select(None))
    );
}
#[test]
fn different_contract_view_layout_or_generation_never_inherits_old_local_value() {
    let _com = Apartment::new();
    let base = focus(scene("slider", json!(2)));
    for case in 0..5 {
        let mut c = Cache::default();
        c.set(base.clone(), Value::Slider(8.), (0, 0)).unwrap();
        let mut next = base.clone();
        match case {
            0 => next.anchor.generation += 1,
            1 => next.anchor.view.attachment_id = "other".into(),
            2 => next.anchor.placement = "other".into(),
            3 => next.rect[2] += 1,
            _ => next.contract[0] ^= 1,
        }
        assert_eq!(c.value(&next), Some(Value::Slider(2.)));
        assert!(c.snapshot().is_empty());
    }
}
#[test]
fn preparation_gap_is_passive_and_retirement_or_clear_releases_every_entry() {
    let _com = Apartment::new();
    let f = focus(scene("switch", json!(false)));
    let mut c = Cache::default();
    c.set(f.clone(), Value::Switch(true), (0, 0)).unwrap();
    let stable = c.snapshot();
    assert!(!c.reconcile(|_| None, |_| true));
    assert!(Rc::ptr_eq(&stable, &c.snapshot()));
    assert!(c.reconcile(|_| None, |_| false));
    assert!(c.snapshot().is_empty());
    c.set(f, Value::Switch(true), (0, 0)).unwrap();
    assert!(c.clear());
    assert!(!c.clear());
}
#[test]
fn shared_field_budget_and_type_range_checks_do_not_evict_existing_values() {
    let _com = Apartment::new();
    let f = focus(scene("slider", json!(2)));
    let mut c = Cache::default();
    for text in [(256, 0), (0, 64)] {
        assert!(c.set(f.clone(), Value::Slider(8.), text).is_err());
        assert!(c.snapshot().is_empty());
    }
    for invalid in [
        Value::Switch(true),
        Value::Slider(f64::NAN),
        Value::Slider(11.),
        Value::Slider(3.),
    ] {
        assert!(c.set(f.clone(), invalid, (0, 0)).is_err());
    }
    c.set(f.clone(), Value::Slider(8.), (255, 63)).unwrap();
    c.set(f.clone(), Value::Slider(10.), (255, 63)).unwrap();
    assert_eq!(c.value(&f), Some(Value::Slider(10.)));
    let mut other = scene("switch", json!(false));
    other["id"] = "other".into();
    assert!(c.set(focus(other), Value::Switch(true), (255, 63)).is_err());
    assert_eq!(c.counts(&f.anchor.view), (1, 1));
}
#[test]
fn handlers_and_disabled_nodes_never_become_local_drafts_or_network_events() {
    let _com = Apartment::new();
    for kind in ["switch", "select", "slider"] {
        let mut s = scene(kind, json!(0));
        s["events"] = json!({"change":"set_value"});
        let f = focus(s);
        assert!(source(&f).is_none());
    }
    let mut s = scene("select", json!("a"));
    let f = focus(s.clone());
    let mut c = Cache::default();
    assert!(c.set(f, Value::Select(Some(2)), (0, 0)).is_err());
    s["props"]["disabled"] = json!(true);
    let scene = crate::surface_scene::Scene::parse(&serde_json::to_vec(&s).unwrap()).unwrap();
    assert!(source_node(scene.root()).is_none());
}
