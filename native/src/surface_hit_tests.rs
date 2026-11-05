//! 命中快照按绘制顺序、圆角与裁剪查找，不承担授权或事件发送。
use super::*;
use crate::surface_presentation::Presentation;
fn prepare(json: &[u8]) -> Presentation {
    Presentation::new(Scene::parse(json).unwrap(), base(), context(), [64., 64.]).unwrap()
}
fn binding() -> crate::surface_live_binding::Binding {
    crate::surface_live_binding::Binding::from_mount(br#"{"instance":{"descriptor":{"instanceId":"instance:test","generation":7},"attachments":{"attachment:test":{"descriptor":{"attachmentId":"attachment:test","instanceId":"instance:test","hookNodeId":"node:test","deviceId":"viewer:test"},"lifecycle":"active","lifecycleRevision":3}}}}"#, "https://localhost:1234", "viewer:test", "node:test").unwrap()
}
#[test]
fn click_batch_uses_current_route_and_preserves_order_revision_and_unattempted_state() {
    let view = prepare(br#"{"id":"root","type":"view","layout":{"height":64},"events":{"click":"outer"},"children":[{"id":"child","type":"view","layout":{"height":64},"events":{"click":"inner"}}]}"#);
    let events = view
        .click_events([1., 1.], &binding(), 57, Default::default())
        .unwrap();
    assert_eq!(events.len(), 2);
    let actions: Vec<_> = events
        .iter()
        .map(|event| {
            assert!(!event.attempted());
            assert_eq!(event.revision(), 57);
            let body: serde_json::Value = serde_json::from_slice(event.bytes()).unwrap();
            body["action"].as_str().unwrap().to_owned()
        })
        .collect();
    assert_eq!(actions, ["inner", "outer"]);
    assert_ne!(events[0].id(), events[1].id());
    assert!(view
        .click_events([-1., 1.], &binding(), 57, Default::default())
        .unwrap()
        .is_empty());
}
#[test]
fn click_batch_rejects_combined_payload_budget_without_returning_partial_actions() {
    let payload = "x".repeat(40 * 1024);
    let json = serde_json::to_vec(
        &serde_json::json!({"id":"root","type":"view","layout":{"height":64},
        "events":{"click":"outer"},"props":{"eventPayload":payload},"children":[{
            "id":"child","type":"view","layout":{"height":64},"events":{"click":"inner"},
            "props":{"eventPayload":payload}
        }]}),
    )
    .unwrap();
    let view = prepare(&json);
    assert_eq!(view.click_route([1., 1.]).count(), 2);
    assert!(view
        .click_events([1., 1.], &binding(), 57, Default::default())
        .is_err());
}
#[test]
fn topmost_hit_respects_own_rounding_and_hidden_nodes() {
    let view = prepare(br#"{"id":"root","type":"stack","layout":{"height":64},"children":[{"id":"back","type":"view"},{"id":"circle","type":"view","style":{"borderRadius":32}},{"id":"hidden","type":"view","props":{"visible":false}}]}"#);
    assert_eq!(view.hit_test([1., 1.]), Some("back"));
    assert_eq!(view.hit_test([32., 32.]), Some("circle"));
    assert_eq!(view.hit_test([64., 32.]), None);
    assert_eq!(view.hit_test([f32::NAN, 0.]), None);
}
#[test]
fn ancestor_padding_round_clip_excludes_child_but_keeps_parent_border_hit() {
    let view = prepare(br#"{"id":"root","type":"view","layout":{"height":64,"position":"relative","overflowX":"hidden"},"style":{"borderWidth":8,"borderRadius":24},"children":[{"id":"child","type":"view","layout":{"position":"absolute","left":0,"top":0,"width":64,"height":64}}]}"#);
    assert_eq!(view.hit_test([10., 10.]), Some("root"));
    assert_eq!(view.hit_test([32., 32.]), Some("child"));
    assert_eq!(view.hit_test([1., 1.]), None);
}
#[test]
fn transparent_content_is_hittable_and_visible_overflow_is_not_parent_clipped() {
    let view = prepare(br#"{"id":"root","type":"column","layout":{"height":64},"children":[{"id":"parent","type":"column","layout":{"width":16,"height":32},"style":{"borderRadius":8},"children":[{"id":"wide","type":"view","layout":{"width":64,"height":32},"style":{"opacity":0}}]}]}"#);
    assert_eq!(view.hit_test([40., 16.]), Some("wide"));
}
#[test]
fn hit_snapshot_reflows_atomically_and_hidden_replacement_clears_targets() {
    let mut view = prepare(br#"{"id":"root","type":"view","layout":{"width":"50%","height":32}}"#);
    assert_eq!(view.hit_test([30., 10.]), Some("root"));
    assert!(view
        .replace(
            Scene::parse(br#"{"id":"bad","type":"image"}"#).unwrap(),
            base(),
            context(),
            [64., 64.]
        )
        .is_err());
    assert_eq!(view.hit_test([30., 10.]), Some("root"));
    view.reflow(base(), context(), [32., 64.]).unwrap();
    assert_eq!(view.hit_test([30., 10.]), None);
    view.replace(
        Scene::parse(br#"{"id":"hidden","type":"view","props":{"visible":false}}"#).unwrap(),
        base(),
        context(),
        [64., 64.],
    )
    .unwrap();
    assert_eq!(view.hit_test([1., 1.]), None);
}

#[test]
fn click_routes_follow_only_committed_scene_and_topmost_hit() {
    let mut view = prepare(br#"{"id":"root","type":"stack","layout":{"height":64},"events":{"click":"root-action"},"children":[{"id":"back","type":"view","events":{"click":"back-action"}},{"id":"front","type":"view","events":{"click":"front-action"}}]}"#);
    let actions = |view: &Presentation| {
        view.click_route([10., 10.])
            .map(|entry| entry.action.to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(actions(&view), ["front-action", "root-action"]);
    assert!(view.click_route([64., 10.]).next().is_none());
    assert!(view
        .replace(
            Scene::parse(br#"{"id":"bad","type":"image","events":{"click":"bad-action"}}"#)
                .unwrap(),
            base(),
            context(),
            [64., 64.]
        )
        .is_err());
    assert_eq!(actions(&view), ["front-action", "root-action"]);
    view.replace(
        Scene::parse(br#"{"id":"root","type":"view","layout":{"height":64},"events":{"click":"new-action"}}"#).unwrap(),
        base(), context(), [64., 64.]
    ).unwrap();
    assert_eq!(actions(&view), ["new-action"]);
}
