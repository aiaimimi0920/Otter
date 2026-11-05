//! 手写数值/事件oracle；模型、尾合并、依赖失败、几何/合同与预览不改权威scene。
use super::*;
use crate::tile_surface_value::tests::{fixture, presentation};
use serde_json::{json, Value as Json};
#[path = "tile_surface_slider_wait_tests.rs"]
mod waits;
pub(crate) fn scene() -> Json {
    json!({"id":"field","type":"slider","layout":{"width":64,"height":64},
        "props":{"min":-10,"max":10,"step":2,"value":0},"events":{"input":"adjust","change":"save"}})
}
fn field(f: &crate::tile_surface_actions::tests::Fixture) -> Field {
    hit(&f.frame, &f.art, &presentation(f), [0, 0]).unwrap()
}
fn event(c: &Command) -> &Json {
    let Action::Event(v) = &c.kind else { panic!() };
    v
}
fn validates(f: &crate::tile_surface_actions::tests::Fixture, c: &mut Command) -> bool {
    crate::tile_surface_action_dispatch::validate(c, &f.state, Some(&f.art), &f.job, &f.shared)
}
#[test]
fn drag_uses_source_resolution_track_step_and_exact_input_then_commit_wire() {
    let f = fixture(scene());
    let mut drag = Drag::new(field(&f));
    assert!(drag.commit().unwrap().is_none());
    // 2px输出映射64px canonical；像素中心16/48，轨道5..59，独立算得-6/6。
    for (pixel, value) in [([0, 0], -6), ([1, 0], 6), ([-100, 0], -10), ([100, 0], 10)] {
        let mut c = drag.sample(&f.frame, pixel).unwrap().unwrap();
        assert_eq!(event(&c)["payload"], json!({"value":value as f64}));
        assert_eq!(event(&c)["event"], "input");
        assert_eq!(event(&c)["class"], "continuous");
        assert!(validates(&f, &mut c));
    }
    assert!(drag.sample(&f.frame, [100, 0]).unwrap().is_none());
    let mut c = drag.commit().unwrap().unwrap();
    assert_eq!(event(&c)["event"], "change");
    assert_eq!(event(&c)["class"], "commit");
    assert_eq!(event(&c)["action"], "save");
    assert_eq!(event(&c)["baseRevision"], 1);
    assert_eq!(event(&c)["instanceId"], "art");
    assert_eq!(event(&c)["attachmentId"], "view");
    assert!(validates(&f, &mut c));
}
#[test]
fn only_slider_value_may_change_while_queued_and_forged_values_never_gain_authority() {
    let f = fixture(scene());
    let mut d = Drag::new(field(&f));
    d.sample(&f.frame, [1, 0]).unwrap();
    let mut s = scene();
    s["props"]["value"] = json!(6);
    let next = fixture(s.clone());
    assert!(field(&f).same(&field(&next)));
    assert!(validates(&next, &mut d.commit().unwrap().unwrap()));
    for (key, value) in [
        ("step", json!(1)),
        ("min", json!(-12)),
        ("disabled", json!(true)),
    ] {
        let mut changed = s.clone();
        changed["props"][key] = value;
        assert!(!validates(
            &fixture(changed),
            &mut d.commit().unwrap().unwrap()
        ));
    }
    for (key, value) in [
        ("payload", json!({"value":5})),
        ("payload", json!({"value":100})),
        ("payload", json!({"value":null})),
        ("payload", json!({"value":6,"extra":0})),
        ("action", json!("other")),
        ("class", json!("continuous")),
        ("modifiers", json!({})),
    ] {
        let mut c = d.commit().unwrap().unwrap();
        let Action::Event(e) = &mut c.kind else {
            panic!()
        };
        e[key] = value;
        assert!(!validates(&f, &mut c), "{key}");
    }
    let mut changed = scene();
    changed["events"] = json!({});
    let readonly = fixture(changed);
    let local = hit(
        &readonly.frame,
        &readonly.art,
        &presentation(&readonly),
        [0, 0],
    )
    .unwrap();
    let mut drag = Drag::new(local);
    assert!(drag.sample(&readonly.frame, [1, 0]).unwrap().is_none());
    assert_eq!(drag.value, 6.);
    assert!(drag.commit().unwrap().is_none());
}
#[test]
fn tail_only_coalescing_preserves_commit_barrier_and_never_exceeds_original_32_slots() {
    let f = fixture(scene());
    let mut d = Drag::new(field(&f));
    let now = Instant::now();
    let mut q = f.shared.lock();
    for i in 0..1000 {
        let pixel = if i % 2 == 0 { [0, 0] } else { [1, 0] };
        assert!(q
            .actions
            .offer(vec![d.sample(&f.frame, pixel).unwrap().unwrap()], now));
    }
    assert_eq!(q.actions.pending.len(), 1);
    assert_eq!(
        event(q.actions.pending.front().unwrap())["payload"],
        json!({"value":6.0})
    );
    assert!(q.actions.offer(vec![d.commit().unwrap().unwrap()], now));
    assert!(q
        .actions
        .offer(vec![d.sample(&f.frame, [-100, 0]).unwrap().unwrap()], now));
    assert_eq!(q.actions.pending.len(), 3); // 不跨commit合并。
    q.actions.clear();
    q.actions.observe(f.state.clone(), "o", "d", now);
    d = Drag::new(field(&f));
    d.sample(&f.frame, [1, 0]).unwrap();
    for _ in 0..32 {
        assert!(q.actions.offer(vec![d.commit().unwrap().unwrap()], now));
    }
    assert!(!q.actions.offer(vec![d.commit().unwrap().unwrap()], now));
    assert_eq!(q.actions.pending.len(), 32);
}
#[test]
fn cancellation_rejection_and_expired_prefix_discard_only_the_same_edit_suffix() {
    for case in 0..3 {
        let f = fixture(scene());
        let mut d = Drag::new(field(&f));
        let mut other = Drag::new(field(&f));
        let now = Instant::now();
        let mut first = d.sample(&f.frame, [1, 0]).unwrap().unwrap();
        if case == 2 {
            first.until = now;
        }
        let mut s = f.shared.lock();
        if case != 2 {
            assert!(s.actions.offer(vec![first], now));
        } else {
            s.actions.pending.push_back(first);
        }
        if case == 2 {
            s.actions.pending.push_back(d.commit().unwrap().unwrap());
            s.actions
                .pending
                .push_back(other.sample(&f.frame, [0, 0]).unwrap().unwrap());
        } else {
            assert!(s.actions.offer(vec![d.commit().unwrap().unwrap()], now));
            assert!(s
                .actions
                .offer(vec![other.sample(&f.frame, [0, 0]).unwrap().unwrap()], now));
        }
        match case {
            0 => s.actions.cancel_edit(&d.edit),
            1 => {
                let c = s.actions.take(&f.state, now).unwrap();
                let mut a = crate::tile_surface_actions::tests::ack_value(
                    event(&c)["eventId"].as_str().unwrap(),
                    "failed",
                );
                a["accepted"] = json!(false);
                let ack = crate::surface_action_ack::Ack::parse_identity(
                    &serde_json::to_vec(&a).unwrap(),
                    "art",
                    event(&c)["eventId"].as_str().unwrap(),
                )
                .unwrap();
                s.actions.attempted(&c, &ack, now);
            }
            _ => {
                s.actions.take(&f.state, now);
            }
        }
        assert!(s
            .actions
            .pending
            .iter()
            .all(|c| c.edit.as_ref().is_none_or(|e| !Arc::ptr_eq(e, &d.edit))));
        assert!(
            case == 2
                || s.actions
                    .pending
                    .iter()
                    .any(|c| c.edit.as_ref().is_some_and(|e| Arc::ptr_eq(e, &other.edit)))
        );
    }
}
#[test]
fn preview_is_paint_only_and_stale_host_or_freeze_never_validates_old_commit() {
    let f = fixture(scene());
    let mut p = presentation(&f);
    p.embed_controls().unwrap();
    let before = p.scene().root().props()["value"].clone();
    assert!(p.preview_slider("field", Some(8.)));
    assert!(
        matches!(p.embedded_control("field"),Some(crate::surface_embedded_control::Paint::Slider{fraction,..}) if (*fraction-0.9).abs()<0.0001)
    );
    assert_eq!(p.scene().root().props()["value"], before);
    assert!(p.preview_slider("field", None));
    assert!(
        matches!(p.embedded_control("field"),Some(crate::surface_embedded_control::Paint::Slider{fraction,..}) if *fraction==0.5)
    );
    let mut d = Drag::new(field(&f));
    d.sample(&f.frame, [1, 0]).unwrap();
    f.shared.lock().frozen = true;
    assert!(!validates(&f, &mut d.commit().unwrap().unwrap()));
    f.shared.lock().frozen = false;
    f.shared.stop();
    assert!(!validates(&f, &mut d.commit().unwrap().unwrap()));
}

fn activity(
    f: &crate::tile_surface_actions::tests::Fixture,
    pending: Json,
    failure: Json,
) -> Arc<crate::tile_surface_wire::State> {
    let mut v = crate::tile_surface_actions::tests::value(&f.job.key, "art");
    v["snapshot"]["scene"] = scene();
    v["pending"] = pending;
    v["failure"] = failure;
    Arc::new(
        crate::tile_surface_wire::State::parse(
            &serde_json::to_vec(&v).unwrap(),
            &f.state.view.binding,
            "art",
            Some(&f.state),
        )
        .unwrap(),
    )
}
#[test]
fn failed_inflight_input_with_empty_queue_cannot_revive_on_next_move_or_commit() {
    for status in ["running", "failed", "cancelled", "interrupted", "unknown"] {
        let f = fixture(scene());
        let mut drag = Drag::new(field(&f));
        let now = Instant::now();
        let mut s = f.shared.lock();
        assert!(s
            .actions
            .offer(vec![drag.sample(&f.frame, [1, 0]).unwrap().unwrap()], now));
        let c = s.actions.take(&f.state, now).unwrap();
        assert!(s.actions.pending.is_empty());
        assert!(s.actions.preview_pending(&drag.edit, &f.frame, "art"));
        s.actions.attempted(&c, &f.ack(&c, status), now);
        if status == "running" {
            let failed = activity(
                &f,
                json!([]),
                json!({"requestId":"request","code":"wall_surface_action_failed"}),
            );
            s.actions.observe(failed, "origin", "device", now);
        }
        assert!(!drag.edit.active(), "{status}");
        assert!(drag.sample(&f.frame, [0, 0]).is_err());
        assert!(drag.commit().is_err());
        assert!(!s.actions.preview_pending(&drag.edit, &f.frame, "art"));
    }
}
#[test]
fn cancelling_accepted_input_immediately_drops_only_its_change_and_click_suffix() {
    let f = fixture(scene());
    let mut drag = Drag::new(field(&f));
    let mut other = Drag::new(field(&f));
    let now = Instant::now();
    let mut s = f.shared.lock();
    assert!(s
        .actions
        .offer(vec![drag.sample(&f.frame, [1, 0]).unwrap().unwrap()], now));
    let c = s.actions.take(&f.state, now).unwrap();
    let event_id = event(&c)["eventId"].as_str().unwrap();
    s.actions.attempted(&c, &f.ack(&c, "running"), now);
    let waiting = activity(
        &f,
        json!([{"ack":crate::tile_surface_actions::tests::ack_value(event_id,"running"),"actionId":"adjust","cancelable":true}]),
        Json::Null,
    );
    s.actions.observe(waiting, "origin", "device", now);
    let mut click = drag.commit().unwrap().unwrap();
    if let Action::Event(e) = &mut click.kind {
        e["event"] = json!("click");
    }
    assert!(s.actions.offer(
        vec![
            drag.commit().unwrap().unwrap(),
            click,
            other.sample(&f.frame, [0, 0]).unwrap().unwrap()
        ],
        now
    ));
    let cancel = Command {
        anchor: c.anchor.clone(),
        kind: Action::Cancel {
            request: "request".into(),
            event: event_id.into(),
        },
        until: c.until,
        edit: None,
    };
    assert!(s.actions.offer(vec![cancel], now));
    assert!(!drag.edit.active());
    assert!(other.edit.active());
    assert_eq!(s.actions.pending.len(), 2);
    assert!(matches!(
        s.actions.pending.back().unwrap().kind,
        Action::Cancel { .. }
    ));
    assert!(s
        .actions
        .pending
        .iter()
        .all(|c| c.edit.as_ref().is_none_or(|e| !Arc::ptr_eq(e, &drag.edit))));
}
#[test]
fn successful_input_tracking_is_bounded_to_active_request_and_preview_retires_after_fresh_state() {
    let f = fixture(scene());
    let mut drag = Drag::new(field(&f));
    let now = Instant::now();
    let mut s = f.shared.lock();
    assert!(s
        .actions
        .offer(vec![drag.sample(&f.frame, [1, 0]).unwrap().unwrap()], now));
    let c = s.actions.take(&f.state, now).unwrap();
    s.actions.attempted(&c, &f.ack(&c, "running"), now);
    assert!(s.actions.preview_pending(&drag.edit, &f.frame, "art"));
    s.actions.observe(f.state.clone(), "origin", "device", now);
    assert!(s.actions.accepted.is_empty());
    assert!(!s.actions.preview_pending(&drag.edit, &f.frame, "art"));
    assert!(drag.edit.active());
    // 同一手势已成功的前缀允许继续；失败/取消由另两回归证明不可复活。
    assert!(drag.sample(&f.frame, [0, 0]).unwrap().is_some());
}
#[test]
fn input_only_and_change_only_keep_original_declared_event_contract() {
    for name in ["input", "change"] {
        let mut v = scene();
        v["events"] = json!({});
        v["events"][name] = json!("run");
        let f = fixture(v);
        let mut d = Drag::new(field(&f));
        let input = d.sample(&f.frame, [1, 0]).unwrap();
        let change = d.commit().unwrap();
        assert_eq!(input.is_some(), name == "input");
        assert_eq!(change.is_some(), name == "change");
        let mut c = input.or(change).unwrap();
        assert!(validates(&f, &mut c));
    }
}
#[test]
fn approved_input_keeps_dependency_tracking_until_terminal_or_expiry() {
    for approve in [false, true] {
        let f = fixture(scene());
        let mut d = Drag::new(field(&f));
        let now = Instant::now();
        let mut s = f.shared.lock();
        let input = d.sample(&f.frame, [1, 0]).unwrap().unwrap();
        let id = event(&input)["eventId"].as_str().unwrap();
        s.actions
            .attempted(&input, &f.ack(&input, "awaiting_confirmation"), now);
        let mut cv = crate::tile_surface_actions::tests::confirmation(id, &f.job.key.endpoint);
        cv["actionId"] = json!("adjust");
        let confirmation = serde_json::from_value(cv).unwrap();
        let decision = Command {
            anchor: input.anchor.clone(),
            kind: Action::Decision {
                confirmation: Box::new(confirmation),
                approved: approve,
            },
            until: input.until,
            edit: None,
        };
        s.actions
            .attempted(&decision, &f.ack(&input, "running"), now);
        assert_eq!(d.edit.active(), approve);
        assert_eq!(s.actions.accepted.len(), usize::from(approve));
        if approve {
            // 首次未终态GET保原请求，明确取消仍能找到Edit。
            let pending = activity(
                &f,
                json!([{"ack":crate::tile_surface_actions::tests::ack_value(id,"running"),"actionId":"adjust","cancelable":true}]),
                Json::Null,
            );
            s.actions.observe(pending, "origin", "device", now);
            assert_eq!(s.actions.accepted.len(), 1);
            s.actions.observe(
                f.state.clone(),
                "origin",
                "device",
                now + Duration::from_secs(121),
            );
            assert!(!d.edit.active());
            assert!(s.actions.accepted.is_empty());
        }
    }
}
