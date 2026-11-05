//! 独立声明/事件oracle；复用Wall夹具，验证实际hit、值合同和新鲜派发围栏。
use super::*;
use crate::tile_surface_actions::{
    tests::{value, Fixture},
    Kind as EventKind,
};
use serde_json::{json, Value as Json};
pub(crate) fn fixture(scene: Json) -> Fixture {
    let mut f = Fixture::new("art");
    let mut wire = value(&f.job.key, "art");
    wire["snapshot"]["scene"] = scene;
    f.state = Arc::new(
        crate::tile_surface_wire::State::parse(
            &serde_json::to_vec(&wire).unwrap(),
            &f.state.view.binding,
            "art",
            None,
        )
        .unwrap(),
    );
    f.art = Arc::new(Art {
        view: f.state.view.clone(),
        generation: f.state.generation,
        snapshot: f.state.snapshot.clone(),
        size: f.state.size,
        images: Default::default(),
    });
    let frame = Frame {
        key: f.frame.key.clone(),
        layout: f.frame.layout.clone(),
        size: f.frame.size,
        images: f.frame.images.clone(),
        arts: [("art".into(), f.art.clone())].into(),
        live_until: None,
    };
    f.shared.publish(1, frame);
    f.frame = f.shared.lock().frame.clone().unwrap();
    f.shared.confirm(&f.frame, Instant::now());
    f.shared.lock().actions.observe(
        f.state.clone(),
        "https://localhost",
        "device",
        Instant::now(),
    );
    f
}
pub(crate) fn switch() -> Json {
    json!({"id":"field","type":"switch","props":{"value":false},"events":{"change":"set_flag"}})
}
pub(crate) fn select() -> Json {
    json!({"id":"field","type":"select","layout":{"width":64,"height":64},"props":{"value":"a","options":[
        {"value":"a","label":"A"},{"value":"b","label":"B & C\t新\n选项"},{"value":"b","label":"duplicate"}]},"events":{"change":"set_choice"}})
}
pub(crate) fn presentation(f: &Fixture) -> Presentation {
    Presentation::new(
        Scene::parse(&f.art.snapshot.scene).unwrap(),
        crate::surface_viewport::style(),
        crate::surface_viewport::context([64., 64.]),
        [64., 64.],
    )
    .unwrap()
}
pub(crate) fn field(f: &Fixture) -> Field {
    hit(
        &f.frame,
        &f.art,
        &presentation(f),
        [0, 0],
        Default::default(),
    )
    .unwrap()
}
fn event(c: &Command) -> &Json {
    let EventKind::Event(v) = &c.kind else {
        panic!()
    };
    v
}
fn validate(f: &Fixture, c: &mut Command) -> bool {
    crate::tile_surface_action_dispatch::validate(c, &f.state, Some(&f.art), &f.job, &f.shared)
}
#[test]
fn boolean_change_and_select_encode_real_wall_identity_without_mounting_or_local_truth() {
    for (scene, selected, action, expected) in [
        (switch(), None, "set_flag", json!(true)),
        (select(), Some(1), "set_choice", json!("b")),
    ] {
        let f = fixture(scene);
        let mut commands = field(&f).commands(selected).unwrap();
        assert_eq!(commands.len(), 1);
        let c = &mut commands[0];
        let e = event(c);
        assert_eq!(e["instanceId"], "art");
        assert_eq!(e["attachmentId"], "view");
        assert_eq!(e["nodeId"], "field");
        assert_eq!(e["event"], "change");
        assert_eq!(e["class"], "commit");
        assert_eq!(e["action"], action);
        assert_eq!(e["payload"], json!({"value":expected}));
        assert!(e.get("modifiers").is_none());
        let id = e["eventId"].clone();
        assert!(validate(&f, c));
        assert_eq!(event(c)["eventId"], id);
        assert_eq!(f.art.snapshot.scene, f.state.snapshot.scene);
    }
}
#[test]
fn selection_cancel_same_index_out_of_range_and_empty_options_never_invent_a_value() {
    let f = fixture(select());
    assert!(field(&f).commands(None).unwrap().is_empty());
    assert!(field(&f).commands(Some(0)).unwrap().is_empty());
    assert!(field(&f).commands(Some(99)).is_err());
    assert_eq!(
        event(&field(&f).commands(Some(2)).unwrap()[0])["payload"],
        json!({"value":"b"})
    );
    let mut s = select();
    s["props"]["value"] = json!("missing");
    s["props"]["options"] = json!([]);
    let f = fixture(s);
    assert!(field(&f).commands(Some(0)).is_err());
}
#[test]
fn hidden_disabled_and_noninteractive_placements_do_not_gain_value_input() {
    for mut scene in [switch(), select()] {
        for property in ["disabled", "visible"] {
            let original = scene.clone();
            match property {
                "disabled" => scene["props"][property] = json!(true),
                "visible" => scene["props"][property] = json!(false),
                _ => unreachable!(),
            }
            let f = fixture(scene);
            assert!(hit(
                &f.frame,
                &f.art,
                &presentation(&f),
                [0, 0],
                Default::default()
            )
            .is_none());
            scene = original;
        }
    }
    let f = fixture(select());
    let mut frame = Frame {
        key: f.frame.key.clone(),
        layout: f.frame.layout.clone(),
        size: f.frame.size,
        images: f.frame.images.clone(),
        arts: f.frame.arts.clone(),
        live_until: None,
    };
    frame.layout.placements[0].interactive = false;
    assert!(hit(
        &frame,
        &f.art,
        &presentation(&f),
        [0, 0],
        Default::default()
    )
    .is_none());
}
#[test]
fn fresh_dispatch_rejects_forged_value_kind_action_class_and_payload() {
    let f = fixture(switch());
    for (key, value) in [
        ("payload", json!({"value":false})),
        ("payload", json!({"value":"true"})),
        ("payload", json!({"value":true,"extra":1})),
        ("class", json!("continuous")),
        ("event", json!("input")),
        ("action", json!("other")),
        ("nodeId", json!("missing")),
        ("modifiers", json!({"ctrlKey":true})),
    ] {
        let mut c = field(&f).commands(None).unwrap().pop().unwrap();
        let EventKind::Event(e) = &mut c.kind else {
            panic!()
        };
        e[key] = value;
        assert!(!validate(&f, &mut c), "{key}");
    }
    let f = fixture(select());
    let mut c = field(&f).commands(Some(1)).unwrap().pop().unwrap();
    let EventKind::Event(e) = &mut c.kind else {
        panic!()
    };
    e["payload"] = json!({"value":"unlisted"});
    assert!(!validate(&f, &mut c));
}
#[test]
fn handlerless_fields_reject_forged_change_even_if_action_is_missing_or_null() {
    for (mut s, value) in [(switch(), json!(true)), (select(), json!("b"))] {
        s["events"] = json!({});
        let f = fixture(s);
        for action in [None, Some(json!(null)), Some(json!("forged"))] {
            let mut event = json!({"nodeId":"field","event":"change","class":"commit","payload":{"value":value}});
            if let Some(action) = action {
                event["action"] = action;
            }
            assert!(!validates(&presentation(&f), [16., 16.], &event));
        }
        assert!(field(&f).commands(Some(1)).unwrap().is_empty());
    }
}
#[test]
fn stale_snapshot_or_source_value_frozen_revoked_and_expired_changes_fail_closed() {
    let f = fixture(switch());
    let mut c = field(&f).commands(None).unwrap().pop().unwrap();
    c.until = Instant::now();
    assert!(!validate(&f, &mut c));
    let mut c = field(&f).commands(None).unwrap().pop().unwrap();
    f.shared.lock().frozen = true;
    assert!(!validate(&f, &mut c));
    f.shared.lock().frozen = false;
    let mut changed = switch();
    changed["props"]["value"] = json!(true);
    let next = fixture(changed);
    assert!(!crate::tile_surface_action_dispatch::validate(
        &mut c,
        &next.state,
        Some(&next.art),
        &next.job,
        &next.shared
    ));
    f.shared.stop();
    assert!(!validate(&f, &mut c));
}
#[test]
fn switch_ancestor_click_is_before_change_and_preserves_modifiers() {
    let f = fixture(
        json!({"id":"root","type":"view","events":{"click":"outer"},"children":[switch()]}),
    );
    let p = presentation(&f);
    let c = hit(
        &f.frame,
        &f.art,
        &p,
        [0, 0],
        crate::surface_click_event::Modifiers {
            ctrl_key: true,
            ..Default::default()
        },
    )
    .unwrap()
    .commands(None)
    .unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!(event(&c[0])["action"], "outer");
    assert_eq!(event(&c[0])["modifiers"]["ctrlKey"], true);
    assert_eq!(event(&c[1])["event"], "change");
}
