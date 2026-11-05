use super::*;
use crate::surface_text::{Align, Whitespace};
fn base() -> Style<'static> {
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
        viewport: [800., 600.],
        container: [200., 200.],
        root_font: 16.,
        font: 999.,
        fraction: None,
    }
}
#[test]
fn tree_inherits_computed_line_height_and_resolves_relative_fonts_once() {
    let scene=Scene::parse(br#"{"id":"root","type":"column","style":{"fontSize":"2em","lineHeight":"150%"},"children":[{"id":"a","type":"text","props":{"text":"Native text"},"style":{"fontSize":"50%"}},{"id":"b","type":"text","props":{"text":"Native text"},"style":{"fontSize":"2em"}}]}"#).unwrap();
    let styles = Styles::build(&scene, base(), context(), &Engine::new().unwrap()).unwrap();
    assert_eq!(styles.get(scene.root()).unwrap().size, 32.);
    let a = &scene.root().children()[0];
    let b = &scene.root().children()[1];
    assert_eq!(styles.get(a).unwrap().size, 16.);
    assert_eq!(styles.get(b).unwrap().size, 64.);
    assert_eq!(styles.get(a).unwrap().line_height, Some(48.));
    let engine = Engine::new().unwrap();
    let actual = styles.layout_text(&engine, b, 1000., 1000.).unwrap();
    let expected = engine
        .layout(
            "Native text",
            Style {
                size: 64.,
                line_height: Some(48.),
                ..base()
            },
            1000.,
            1000.,
        )
        .unwrap();
    assert_eq!(actual.size(), expected.size());
}
#[test]
fn style_table_rejects_replacement_scene_even_when_ids_match() {
    let bytes = br#"{"id":"root","type":"text","props":{"text":"Original"}}"#;
    let original = Scene::parse(bytes).unwrap();
    let replacement = Scene::parse(bytes).unwrap();
    let styles = Styles::build(&original, base(), context(), &Engine::new().unwrap()).unwrap();
    assert!(styles.get(replacement.root()).is_none());
    assert!(styles
        .layout_text(&Engine::new().unwrap(), replacement.root(), 100., 100.)
        .is_err());
    let invalid =
        Scene::parse(br#"{"id":"root","type":"text","style":{"fontSize":"1000rem"}}"#).unwrap();
    assert!(Styles::build(&invalid, base(), context(), &Engine::new().unwrap()).is_err());
}
