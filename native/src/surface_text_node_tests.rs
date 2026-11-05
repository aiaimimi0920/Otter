use super::*;
use crate::surface_scene::Scene;
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
        axis: 200.,
        viewport: [1000., 800.],
        container: [200., 200.],
        root_font: 16.,
        font: 999.,
        fraction: None,
    }
}
fn scene(style: serde_json::Value) -> Scene {
    Scene::parse(&serde_json::to_vec(&serde_json::json!({"id":"text:1","type":"text","props":{"text":"native text native text native text"},"style":style})).unwrap()).unwrap()
}
#[test]
fn scene_text_uses_resolved_inheritance_in_actual_directwrite_layout() {
    let line = scene(serde_json::json!({"fontSize":"2em","lineHeight":"150%"}));
    assert_eq!(
        resolve_style(line.root(), style(), context()).line_height,
        Some(48.)
    );
    let percentage = scene(serde_json::json!({"fontSize":"200%"}));
    assert_eq!(
        resolve_style(percentage.root(), style(), context()).size,
        32.
    );
    let node = scene(
        serde_json::json!({"fontSize":"2em","fontWeight":1200,"whiteSpace":"nowrap","textAlign":"right"}),
    );
    let resolved = resolve_style(node.root(), style(), context());
    assert_eq!(resolved.size, 32.);
    assert_eq!(resolved.weight, 900);
    assert!(matches!(resolved.align, Align::End));
    let engine = Engine::new().unwrap();
    let large = engine
        .layout_node(node.root(), style(), context(), 70., 1000.)
        .unwrap();
    let plain = scene(serde_json::json!({"whiteSpace":"nowrap"}));
    let small = engine
        .layout_node(plain.root(), style(), context(), 70., 1000.)
        .unwrap();
    assert_eq!(large.line_count(), 1);
    assert!(large.size()[0] > small.size()[0]);
    let wrapped = scene(serde_json::json!({"whiteSpace":"normal"}));
    assert!(
        engine
            .layout_node(wrapped.root(), style(), context(), 70., 1000.)
            .unwrap()
            .line_count()
            > 1
    );
}
#[test]
fn explicit_normal_resets_inherited_line_spacing_in_actual_directwrite() {
    let mut inherited = style();
    inherited.line_height = Some(80.);
    let node = scene(serde_json::json!({"fontSize":"20px","lineHeight":"normal"}));
    let resolved = resolve_style(node.root(), inherited, context());
    assert_eq!(resolved.line_height, None);
    let engine = Engine::new().unwrap();
    let actual = engine
        .layout_node(node.root(), inherited, context(), 200., 400.)
        .unwrap();
    let expected = engine
        .layout(
            "native text native text native text",
            Style {
                size: 20.,
                ..style()
            },
            200.,
            400.,
        )
        .unwrap();
    assert_eq!(actual.size(), expected.size());
    assert_eq!(actual.baseline(), expected.baseline());
}
#[test]
fn invalid_style_retains_parent_and_nontext_is_not_silently_rendered() {
    let node = scene(
        serde_json::json!({"fontSize":"url(secret)","fontWeight":"bold","whiteSpace":"script","textAlign":"evil"}),
    );
    let resolved = resolve_style(node.root(), style(), context());
    assert_eq!(resolved.size, 16.);
    assert_eq!(resolved.weight, 400);
    assert_eq!(resolved.whitespace, Whitespace::Normal);
    let node = Scene::parse(br#"{"id":"root","type":"view"}"#).unwrap();
    assert!(Engine::new()
        .unwrap()
        .layout_node(node.root(), style(), context(), 100., 100.)
        .is_err());
}
