//! 按钮内容盒与静态样式回归；不以测量成功代替完整键盘交互验收。
use super::*;
use crate::surface_presentation::Presentation;
#[test]
fn surface_button_cursor_distinguishes_disabled_buttons_and_nonbuttons() {
    for disabled in [false, true] {
        let json = format!(
            r#"{{"id":"button","type":"button","props":{{"disabled":{disabled}}},"layout":{{"width":120,"height":60}}}}"#
        );
        let owner = Presentation::new(
            Scene::parse(json.as_bytes()).unwrap(),
            base(),
            context(),
            [200., 100.],
        )
        .unwrap();
        assert_eq!(owner.button_enabled_at([10., 10.]), Some(!disabled));
        assert_eq!(owner.button_enabled_at([150., 80.]), None);
    }
}
#[test]
fn surface_button_focus_order_wraps_and_skips_disabled_or_hidden() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"a","type":"button"},{"id":"hidden","type":"button","props":{"visible":false}},{"id":"disabled","type":"button","props":{"disabled":true}},{"id":"b","type":"button"}]}"#).unwrap();
    let owner = Presentation::new(scene, base(), context(), [200., 200.]).unwrap();
    assert_eq!(owner.next_focus(None, true), Some("b"));
    assert_eq!(owner.next_focus(Some("b"), true), Some("a"));
    assert_eq!(owner.next_focus(Some("a"), true), Some("b"));
    assert_eq!(owner.next_focus(Some("missing"), false), Some("a"));
    assert!(!owner.focusable("hidden"));
    assert!(!owner.focusable("disabled"));
}

#[test]
fn surface_button_label_has_default_insets_and_vertical_centering() {
    let scene = Scene::parse(br#"{"id":"button","type":"button","props":{"label":"Run","text":"ignored"},"layout":{"width":120,"height":60}}"#).unwrap();
    assert_eq!(crate::surface_button::content(scene.root()), "Run");
    assert_eq!(
        crate::surface_button::insets(scene.root(), 120., context()).start(),
        [13., 7.]
    );
    let owner = Presentation::new(scene, base(), context(), [200., 100.]).unwrap();
    let text = &owner.text().items()[0];
    assert_eq!(text.allocated(), [94., 46.]);
    assert_eq!(text.origin()[0], 13.);
    assert!((text.origin()[1] - (60. - text.layout().size()[1]) / 2.).abs() < 0.01);
    assert_eq!(owner.geometry().nodes()[0].size(), [120., 60.]);
}

#[test]
fn surface_button_defaults_do_not_overwrite_explicit_styles() {
    let scene = Scene::parse(br#"{"id":"button","type":"button","props":{"disabled":true},"layout":{"padding":2,"minHeight":0},"style":{"borderWidth":3,"opacity":0.8,"color":"red"}}"#).unwrap();
    let node = scene.root();
    assert_eq!(node.style()["opacity"], 0.8);
    assert_eq!(node.style()["color"], "red");
    assert_eq!(
        crate::surface_button::insets(node, 120., context()).start(),
        [5., 5.]
    );
    assert_eq!(
        crate::surface_button::minimum(node, 120., context(), 1),
        10.
    );
    let scene =
        Scene::parse(br#"{"id":"button","type":"button","props":{"disabled":true}}"#).unwrap();
    assert_eq!(scene.root().style()["opacity"], 0.48);
    assert_eq!(
        crate::surface_button::minimum(scene.root(), 120., context(), 1),
        30.
    );
    let owner = Presentation::new(scene, base(), context(), [200., 100.]).unwrap();
    assert!(owner.geometry().nodes()[0].size()[1] >= 30.);
}
