//! 原生滚动快照验证：几何、文字与命中一致，双轴、嵌套边界及更新钳位。
#[path = "surface_scroll_cache_tests.rs"]
mod cache_tests;
#[path = "surface_focus_reveal_tests.rs"]
mod focus_reveal;
#[path = "surface_scroll_keyboard_tests.rs"]
mod keyboard_tests;
#[path = "surface_scrollbar_tests.rs"]
mod scrollbar_tests;
use crate::{
    surface_length::Context,
    surface_presentation::Presentation,
    surface_scene::Scene,
    surface_text::{Align, Style, Whitespace},
};
fn style() -> Style<'static> {
    Style {
        family: "Segoe UI",
        size: 16.,
        weight: 400,
        whitespace: Whitespace::Normal,
        align: Align::Start,
        line_height: None,
    }
}
fn context() -> Context {
    Context {
        axis: 100.,
        viewport: [100.; 2],
        container: [100.; 2],
        root_font: 16.,
        font: 16.,
        fraction: None,
    }
}
fn scene(height: u32) -> Scene {
    Scene::parse(serde_json::json!({"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[
        {"id":"content","type":"column","layout":{"width":300,"minWidth":300,"height":height,"minHeight":height},"children":[
            {"id":"text","type":"text","props":{"text":"hello"},"layout":{"height":40,"minHeight":40}},
            {"id":"button","type":"button","props":{"label":"Run"},"layout":{"height":40,"minHeight":40}}
        ]}
    ]}).to_string().as_bytes()).unwrap()
}
fn wheel(p: &mut Presentation, axis: usize, delta: f32) -> bool {
    p.wheel([10., 10.], axis, delta, false).unwrap()
}
fn origin(p: &Presentation, id: &str) -> [f32; 2] {
    p.geometry()
        .nodes()
        .iter()
        .find(|n| n.id() == id)
        .unwrap()
        .origin()
}
#[test]
fn surface_scroll_moves_text_geometry_and_hit_together_and_clamps() {
    let mut p = Presentation::new(scene(300), style(), context(), [100.; 2]).unwrap();
    assert_eq!(p.hit_test([10., 10.]), Some("text"));
    let text_origin = p.text().items()[0].origin();
    assert!(wheel(&mut p, 1, 40.));
    assert_eq!(p.hit_test([10., 10.]), Some("button"));
    assert_eq!(origin(&p, "button")[1], 0.);
    assert_eq!(p.text().items()[0].origin()[1], text_origin[1] - 40.);
    assert!(p.text().items()[0].clip().empty());
    assert!(wheel(&mut p, 0, 50.));
    assert_eq!(origin(&p, "content"), [-50., -40.]);
    assert!(wheel(&mut p, 1, 10000.));
    assert_eq!(origin(&p, "content")[1], -200.);
    assert!(!wheel(&mut p, 1, 1.));
    assert!(!wheel(&mut p, 1, f32::NAN));
    assert!(!wheel(&mut p, 2, 1.));
}
#[test]
fn surface_scroll_replacement_preserves_and_clamps_only_current_nodes() {
    let mut p = Presentation::new(scene(300), style(), context(), [100.; 2]).unwrap();
    assert!(wheel(&mut p, 1, 180.));
    p.replace(scene(150), style(), context(), [100.; 2])
        .unwrap();
    assert_eq!(origin(&p, "content")[1], -50.);
    p.replace(scene(80), style(), context(), [100.; 2]).unwrap();
    assert_eq!(origin(&p, "content")[1], 0.);
}
#[test]
fn surface_scroll_nested_boundary_chains_without_leaking_inner_extent() {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"inner","type":"scroll","layout":{"height":80,"minHeight":80},"children":[{"id":"inside","type":"view","layout":{"height":300,"minHeight":300}}]},{"id":"tail","type":"view","layout":{"height":100,"minHeight":100}}]}"#).unwrap();
    let mut p = Presentation::new(scene, style(), context(), [100.; 2]).unwrap();
    assert!(p.wheel([10., 10.], 1, 1., true).unwrap());
    assert_eq!(origin(&p, "inside")[1], -80.);
    assert!(wheel(&mut p, 1, 1000.));
    assert_eq!(origin(&p, "inner")[1], 0.);
    assert_eq!(origin(&p, "inside")[1], -220.);
    assert!(wheel(&mut p, 1, 1000.));
    assert_eq!(origin(&p, "inner")[1], -80.);
    assert!(!wheel(&mut p, 1, 1000.));
}
#[test]
fn surface_scroll_fractional_distance_is_retained_and_reflow_clamps() {
    let mut p = Presentation::new(scene(300), style(), context(), [100.; 2]).unwrap();
    for _ in 0..120 {
        assert!(wheel(&mut p, 1, 0.125));
    }
    assert_eq!(origin(&p, "content")[1], -15.);
    assert!(wheel(&mut p, 1, -15.));
    assert_eq!(origin(&p, "content")[1], 0.);
}
#[test]
fn surface_scroll_range_includes_visible_child_trailing_border() {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"child","type":"view","layout":{"height":200,"minHeight":200},"style":{"borderWidth":4}}]}"#).unwrap();
    let mut p = Presentation::new(scene, style(), context(), [100.; 2]).unwrap();
    assert!(wheel(&mut p, 1, 1000.));
    assert_eq!(origin(&p, "child")[1], -100.);
}
#[test]
fn surface_scroll_hidden_axis_and_empty_borders_do_not_scroll() {
    for json in [
        r#"{"id":"root","type":"scroll","layout":{"width":100,"height":100,"overflowY":"hidden"},"children":[{"id":"child","type":"view","layout":{"height":300,"minHeight":300}}]}"#,
        r#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"style":{"borderWidth":4}}"#,
    ] {
        let mut p = Presentation::new(
            Scene::parse(json.as_bytes()).unwrap(),
            style(),
            context(),
            [100.; 2],
        )
        .unwrap();
        assert!(!wheel(&mut p, 1, 50.));
    }
}
