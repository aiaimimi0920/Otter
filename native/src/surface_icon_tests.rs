//! Icon 的真实 DirectWrite/窗口与合同边界，不用枚举存在冒充可呈现。
use super::*;
use crate::surface_presentation::Presentation;

fn scene(props: serde_json::Value) -> Scene {
    Scene::parse(
        &serde_json::to_vec(&serde_json::json!({"id":"icon","type":"icon","props":props})).unwrap(),
    )
    .unwrap()
}
#[test]
fn surface_icon_default_empty_and_unicode_content_are_distinct() {
    for (props, expected) in [
        (serde_json::json!({}), "•"),
        (serde_json::json!({"glyph":null}), "•"),
        (serde_json::json!({"glyph":4}), "•"),
        (serde_json::json!({"glyph":""}), ""),
        (serde_json::json!({"glyph":"中→🙂"}), "中→🙂"),
    ] {
        let scene = scene(props);
        assert_eq!(crate::surface_button::content(scene.root()), expected);
        let mut measure = NativeMeasure::new(&scene, base(), context(), [80., 80.]).unwrap();
        let prepared = measure.prepare(&scene, [80., 80.], context()).unwrap();
        assert_eq!(prepared.text().items().len(), 1);
    }
}
#[test]
fn surface_icon_ignores_local_css_and_children_but_inherits_parent_font() {
    let scene = Scene::parse(br##"{"id":"root","type":"column","style":{"fontSize":32},"children":[{"id":"icon","type":"icon","props":{"glyph":"X"},"style":{"fontSize":8000,"opacity":0,"color":"red"},"layout":{"width":9000,"height":9000},"accessibility":{"label":"status"},"children":[{"id":"ignored","type":"image"}]}]}"##).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [80., 80.]).unwrap();
    let prepared = measure.prepare(&scene, [80., 80.], context()).unwrap();
    assert_eq!(prepared.tree().nodes().len(), 2);
    assert!(prepared.tree().nodes()[1].size[1] < 80.);
    let node = &scene.root().children()[0];
    assert_eq!(node.accessibility()["label"], "status");
    assert_eq!(measure.font_size(node), Some(32.));
    assert_eq!(node.style()["color"], "#d9ff38");
    assert!(node.layout().is_empty());
}
#[test]
fn surface_icon_rejects_oversized_or_nul_glyph_without_replacing_frame() {
    let mut presentation = Presentation::new(
        scene(serde_json::json!({"glyph":"X"})),
        base(),
        context(),
        [80., 80.],
    )
    .unwrap();
    for glyph in ["x".repeat(65537), "\0".into()] {
        assert!(presentation
            .replace(
                scene(serde_json::json!({"glyph":glyph})),
                base(),
                context(),
                [80., 80.]
            )
            .is_err());
        assert_eq!(
            crate::surface_button::content(presentation.scene().root()),
            "X"
        );
    }
}
#[test]
fn surface_icon_is_not_focusable_or_an_action_and_scroll_uses_cached_text() {
    let scene = Scene::parse(br#"{"id":"root","type":"scroll","layout":{"width":80,"height":80},"children":[{"id":"content","type":"column","layout":{"height":300,"minHeight":300},"children":[{"id":"icon","type":"icon","props":{"glyph":"X"},"events":{"click":"not-a-button"}}]}]}"#).unwrap();
    let mut presentation = Presentation::new(scene, base(), context(), [80., 80.]).unwrap();
    assert!(
        crate::surface_focus::buttons(presentation.scene(), presentation.geometry()).is_empty()
    );
    let before = crate::surface_text::measure_calls();
    let origin = presentation.text().items()[0].origin();
    assert!(presentation.wheel([5., 5.], 1, 40., false).unwrap());
    assert_eq!(crate::surface_text::measure_calls(), before);
    assert_eq!(
        presentation.text().items()[0].origin(),
        [origin[0], origin[1] - 40.]
    );
}
