//! 双轴条范围、命中遮挡和绝对拖动；不以 GUI 截图替代模型边界。
use super::*;
#[test]
fn surface_scrollbar_short_track_keeps_nonzero_drag_travel() {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":12},"children":[{"id":"child","type":"view","layout":{"height":300,"minHeight":300}}]}"#).unwrap();
    let mut p = Presentation::new(scene, style(), context(), [100.; 2]).unwrap();
    let bar = p.scrollbar_at([95., 2.]).unwrap();
    let drag = bar.drag([95., 2.]).unwrap();
    assert!(p
        .set_scrollbar(&drag.id, drag.axis, drag.value([95., 12.]))
        .unwrap());
    assert_eq!(origin(&p, "child")[1], -288.);
}
#[test]
fn surface_scrollbar_thumb_drag_clamps_without_measuring_or_drifting() {
    let mut p = Presentation::new(scene(300), style(), context(), [100.; 2]).unwrap();
    let bar = p.scrollbar_at([95., 10.]).unwrap();
    assert_eq!(bar.axis, 1);
    assert_eq!(bar.track.bottom, 90.);
    assert_eq!(bar.thumb.bottom, 30.);
    assert_eq!(p.hit_test([95., 10.]), None);
    let drag = bar.drag([95., 10.]).unwrap();
    let count = crate::surface_text::measure_calls();
    for _ in 0..10 {
        assert!(p
            .set_scrollbar(&drag.id, drag.axis, drag.value([95., 70.]))
            .unwrap());
        assert_eq!(origin(&p, "content")[1], -200.);
        assert!(p
            .set_scrollbar(&drag.id, drag.axis, drag.value([95., 10.]))
            .unwrap());
        assert_eq!(origin(&p, "content")[1], 0.);
    }
    assert_eq!(crate::surface_text::measure_calls(), count);
    assert!(!p.set_scrollbar("root", 2, 10.).unwrap());
    assert!(!p.set_scrollbar("root", 1, f32::NAN).unwrap());
    let horizontal = p.scrollbar_at([10., 95.]).unwrap();
    assert_eq!(horizontal.axis, 0);
    assert!(p
        .set_scrollbar("root", 0, horizontal.page_value([80., 95.]))
        .unwrap());
    assert_eq!(origin(&p, "content")[0], -100.);
    // 滚轮落在滚动条上仍交给其所属容器，而非被覆盖的内容。
    assert!(p.wheel([95., 50.], 1, 10., false).unwrap());
}
#[test]
fn surface_scrollbar_hidden_and_empty_axes_have_no_pointer_surface() {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100,"overflowY":"hidden"},"children":[{"id":"child","type":"view","layout":{"height":300,"minHeight":300}}]}"#).unwrap();
    let mut p = Presentation::new(scene, style(), context(), [100.; 2]).unwrap();
    assert!(p.scrollbars().is_empty());
    assert!(!p.set_scrollbar("root", 1, 30.).unwrap());
    assert_eq!(p.hit_test([95., 10.]), Some("child"));
}
#[test]
fn surface_scrollbar_nested_tracks_stay_inside_ancestor_clip() {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"gap","type":"view","layout":{"height":80,"minHeight":80}},{"id":"inner","type":"scroll","layout":{"height":80,"minHeight":80},"children":[{"id":"child","type":"view","layout":{"height":300,"minHeight":300}}]}]}"#).unwrap();
    let p = Presentation::new(scene, style(), context(), [100.; 2]).unwrap();
    let bar = p
        .scrollbars()
        .into_iter()
        .find(|b| b.id == "inner")
        .unwrap();
    assert!(bar.hit([95., 90.]));
    assert!(!bar.hit([95., 110.]));
    assert_eq!(p.scrollbar_at([95., 90.]).unwrap().id, "inner");
}
