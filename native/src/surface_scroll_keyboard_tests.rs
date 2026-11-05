//! 键盘滚动与滚轮共用轴准入/边界传递，复用文字布局而不创建动作。
use super::*;
use crate::surface_scroll::Command::*;
#[test]
fn surface_gesture_pan_rejects_partial_invalid_delta_and_clamps_both_axes() {
    let mut p = Presentation::new(scene(300), style(), context(), [100.; 2]).unwrap();
    assert!(!p.pan("text", [10., f32::NAN]).unwrap());
    assert_eq!(origin(&p, "content"), [0., 0.]);
    assert!(p.pan("text", [20., 30.]).unwrap());
    assert_eq!(origin(&p, "content"), [-20., -30.]);
    assert!(p.pan("text", [10000., 10000.]).unwrap());
    assert_eq!(origin(&p, "content"), [-200., -200.]);
    assert!(!p.pan("removed", [-20., -30.]).unwrap());
}
#[test]
fn surface_scroll_keyboard_steps_pages_edges_and_reuses_measurements() {
    let mut p = Presentation::new(scene(300), style(), context(), [100.; 2]).unwrap();
    let count = crate::surface_text::measure_calls();
    assert!(p.scroll_key(None, Down).unwrap());
    assert_eq!(origin(&p, "content"), [0., -40.]);
    assert!(p.scroll_key(None, PageDown).unwrap());
    assert_eq!(origin(&p, "content"), [0., -140.]);
    assert!(p.scroll_key(None, End).unwrap());
    assert_eq!(origin(&p, "content")[1], -200.);
    assert!(!p.scroll_key(None, End).unwrap());
    assert!(p.scroll_key(None, Home).unwrap());
    assert!(p.scroll_key(None, Right).unwrap());
    assert_eq!(origin(&p, "content"), [-40., 0.]);
    assert!(p.scroll_key(None, Left).unwrap());
    assert!(!p.scroll_key(Some("removed"), Down).unwrap());
    assert_eq!(crate::surface_text::measure_calls(), count);
}
#[test]
fn surface_scroll_keyboard_nested_boundary_chains_but_hidden_does_not_consume() {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100},"children":[{"id":"inner","type":"scroll","layout":{"height":80,"minHeight":80},"children":[{"id":"child","type":"view","layout":{"height":300,"minHeight":300}}]},{"id":"tail","type":"view","layout":{"height":100,"minHeight":100}}]}"#).unwrap();
    let mut p = Presentation::new(scene, style(), context(), [100.; 2]).unwrap();
    assert!(p.scroll_key(Some("child"), End).unwrap());
    assert_eq!(origin(&p, "inner")[1], 0.);
    assert_eq!(origin(&p, "child")[1], -220.);
    assert!(p.scroll_key(Some("child"), End).unwrap());
    assert_eq!(origin(&p, "inner")[1], -80.);
    let hidden = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":100,"height":100,"overflowY":"hidden"},"children":[{"id":"child","type":"view","layout":{"height":300,"minHeight":300}}]}"#).unwrap();
    p.replace(hidden, style(), context(), [100.; 2]).unwrap();
    assert!(!p.scroll_key(Some("child"), Down).unwrap());
}
