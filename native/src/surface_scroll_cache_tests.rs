//! 纯滚动/焦点显露不调用文字测量；场景/尺寸/样式更新使缓存重新准入。
use super::*;
use crate::surface_text::measure_calls;
#[test]
fn surface_scroll_cache_reuses_text_layouts_without_cumulative_translation() {
    let mut p = Presentation::new(scene(300), style(), context(), [100.; 2]).unwrap();
    let initial = p.text().items()[0].origin();
    let count = measure_calls();
    for _ in 0..120 {
        assert!(wheel(&mut p, 1, 0.125));
        assert!(wheel(&mut p, 1, -0.125));
    }
    assert_eq!(measure_calls(), count);
    assert_eq!(p.text().items()[0].origin(), initial);
    assert_eq!(origin(&p, "content"), [0., 0.]);
    assert_eq!(p.hit_test([10., 10.]), Some("text"));
    p.reflow(style(), context(), [100.; 2]).unwrap();
    assert!(measure_calls() > count);
}
#[test]
fn surface_scroll_cache_focus_reveal_does_not_measure_but_style_and_scene_do() {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"height":100},"children":[{"id":"first","type":"button","props":{"label":"First"},"layout":{"height":40,"minHeight":40}},{"id":"gap","type":"view","layout":{"height":160,"minHeight":160}},{"id":"last","type":"button","props":{"label":"Last"},"layout":{"height":40,"minHeight":40}}]}"#).unwrap();
    let mut p = Presentation::new(scene, style(), context(), [100.; 2]).unwrap();
    let count = measure_calls();
    for _ in 0..10 {
        assert_eq!(
            p.reveal_next_focus(Some("first"), false)
                .unwrap()
                .as_deref(),
            Some("last")
        );
        assert_eq!(
            p.reveal_next_focus(Some("last"), true).unwrap().as_deref(),
            Some("first")
        );
    }
    assert_eq!(measure_calls(), count);
    let mut larger = style();
    larger.size = 24.;
    p.reflow(larger, context(), [100.; 2]).unwrap();
    let refreshed = measure_calls();
    assert!(refreshed > count);
    assert!(wheel(&mut p, 1, 20.));
    assert_eq!(measure_calls(), refreshed);
    p.replace(super::scene(200), style(), context(), [100.; 2])
        .unwrap();
    assert!(measure_calls() > refreshed);
    assert_eq!(p.text().items()[0].node_id(), "text");
}
#[test]
fn surface_scroll_cache_failed_reflow_preserves_previous_scrollable_snapshot() {
    let mut p = Presentation::new(scene(300), style(), context(), [100.; 2]).unwrap();
    assert!(wheel(&mut p, 1, 40.));
    let before = p.text().items()[0].origin();
    assert!(p.reflow(style(), context(), [f32::NAN, 100.]).is_err());
    assert_eq!(p.text().items()[0].origin(), before);
    assert!(wheel(&mut p, 1, -40.));
    assert_eq!(origin(&p, "content"), [0., 0.]);
}
#[test]
fn border_sides_scroll_page_reveal_and_cached_text_use_each_inner_edge() {
    let scene=Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"style":{"borderTopWidth":2,"borderRightWidth":4,"borderBottomWidth":6,"borderLeftWidth":8},"children":[{"id":"first","type":"button","props":{"label":"First"},"layout":{"height":40,"minHeight":40}},{"id":"gap","type":"view","layout":{"height":160,"minHeight":160}},{"id":"last","type":"button","props":{"label":"Last"},"layout":{"height":40,"minHeight":40}}]}"#).unwrap();
    let mut p = Presentation::new(scene, style(), context(), [100.; 2]).unwrap();
    assert_eq!(origin(&p, "first"), [8., 2.]);
    let calls = measure_calls();
    assert!(p.wheel([10., 10.], 1, 1., true).unwrap());
    assert_eq!(origin(&p, "first"), [8., -90.]);
    assert!(p.wheel([10., 10.], 1, -1., true).unwrap());
    assert_eq!(origin(&p, "first"), [8., 2.]);
    assert_eq!(
        p.reveal_next_focus(Some("first"), false)
            .unwrap()
            .as_deref(),
        Some("last")
    );
    assert_eq!(origin(&p, "last"), [8., 54.]);
    assert_eq!(
        p.reveal_next_focus(Some("last"), true).unwrap().as_deref(),
        Some("first")
    );
    assert_eq!(origin(&p, "first"), [8., 2.]);
    assert_eq!(measure_calls(), calls);
}
