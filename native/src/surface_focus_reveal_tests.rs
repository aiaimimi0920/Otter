//! 屏外键盘目标、嵌套显露和不可达节点跳过；可见动作资格仍单独检查。
use super::*;
fn presentation(json: &str) -> Presentation {
    Presentation::new(
        Scene::parse(json.as_bytes()).unwrap(),
        style(),
        context(),
        [100.; 2],
    )
    .unwrap()
}
fn next(p: &mut Presentation, current: Option<&str>, reverse: bool) -> Option<String> {
    p.reveal_next_focus(current, reverse).unwrap()
}
#[test]
fn surface_focus_reveal_tab_reaches_offscreen_button_and_reverse_restores_first() {
    let mut p = presentation(
        r#"{"id":"root","type":"scroll","layout":{"height":100},"children":[{"id":"first","type":"button","layout":{"height":40,"minHeight":40}},{"id":"gap","type":"view","layout":{"height":160,"minHeight":160}},{"id":"last","type":"button","layout":{"height":40,"minHeight":40}}]}"#,
    );
    assert!(p.focusable("first"));
    assert!(!p.focusable("last"));
    assert_eq!(next(&mut p, Some("first"), false).as_deref(), Some("last"));
    assert!(p.focusable("last"));
    assert_eq!(origin(&p, "last")[1], 60.);
    assert_eq!(next(&mut p, Some("last"), true).as_deref(), Some("first"));
    assert_eq!(origin(&p, "first")[1], 0.);
}
#[test]
fn surface_focus_reveal_hidden_axis_can_focus_but_still_cannot_wheel() {
    let mut p = presentation(
        r#"{"id":"root","type":"view","layout":{"height":100,"overflowY":"hidden"},"children":[{"id":"gap","type":"view","layout":{"height":160,"minHeight":160}},{"id":"field","type":"input","layout":{"height":32,"minHeight":32}}]}"#,
    );
    assert!(!wheel(&mut p, 1, 100.));
    assert_eq!(next(&mut p, None, false).as_deref(), Some("field"));
    assert_eq!(origin(&p, "field")[1], 68.);
    assert!(!wheel(&mut p, 1, -100.));
}
#[test]
fn surface_focus_reveal_unreachable_target_does_not_move_outer_scroll() {
    let mut p = presentation(
        r#"{"id":"root","type":"scroll","layout":{"height":100},"children":[{"id":"gap","type":"view","layout":{"height":160,"minHeight":160}},{"id":"zero","type":"scroll","layout":{"height":0},"children":[{"id":"button","type":"button","layout":{"height":30,"minHeight":30}}]}]}"#,
    );
    let before = origin(&p, "zero");
    assert!(next(&mut p, None, false).is_none());
    assert_eq!(origin(&p, "zero"), before);
}
#[test]
fn surface_focus_reveal_nested_ancestors_and_zero_clip_skip() {
    let mut p = presentation(
        r#"{"id":"root","type":"scroll","layout":{"height":100},"children":[{"id":"zero","type":"scroll","layout":{"height":0,"minHeight":0},"children":[{"id":"unreachable","type":"button","layout":{"height":30,"minHeight":30}}]},{"id":"gap","type":"view","layout":{"height":160,"minHeight":160}},{"id":"inner","type":"scroll","layout":{"height":80,"minHeight":80},"children":[{"id":"space","type":"view","layout":{"height":150,"minHeight":150}},{"id":"button","type":"button","layout":{"height":30,"minHeight":30}}]}]}"#,
    );
    assert_eq!(next(&mut p, None, false).as_deref(), Some("button"));
    let node = p
        .geometry()
        .nodes()
        .iter()
        .find(|n| n.id() == "button")
        .unwrap();
    assert!(!node.clip().empty());
    assert!(node.origin()[1] >= 0. && node.origin()[1] + node.size()[1] <= 100.);
}
