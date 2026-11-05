use super::*;
use crate::{
    surface_scene::Scene, surface_text::Engine, surface_text_styles::Styles, surface_viewport,
};

#[test]
fn font_family_parser_has_bounded_names_quotes_and_generic_identity() {
    let parsed = parse(" 'Segoe UI' , Microsoft   YaHei, MONOSPACE, 'serif', 微软雅黑 ").unwrap();
    assert_eq!(
        parsed.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(),
        [
            "Segoe UI",
            "Microsoft YaHei",
            "monospace",
            "serif",
            "微软雅黑"
        ]
    );
    assert!(parsed[2].generic && !parsed[3].generic);
    for bad in [
        "", " ", ",serif", "serif,", "'serif", "'a'junk", "inherit", "DEFAULT", "12px", "a -b",
        "url(x)", "a;b", "a\\b", "'a,b'", "a\0b", "🙂",
    ] {
        assert!(parse(bad).is_err(), "{bad:?}");
    }
    assert!(parse(&"a".repeat(32)).is_err());
    assert!(parse(&["a"; 9].join(",")).is_err());
    assert!(parse(&vec!["一".repeat(31); 3].join(",")).is_err());
    let mut boundary = vec![format!("\"{}\"", "a".repeat(29)); 8];
    boundary[0] = format!("\"{}\"", "a".repeat(30));
    let boundary = boundary.join(",");
    assert_eq!(boundary.len(), 256);
    assert!(parse(&boundary).is_ok());
}

#[test]
fn font_family_native_selection_is_ordered_lazy_cached_and_bounded() {
    let engine = Engine::new().unwrap();
    assert!(engine.families.borrow().collection.is_none());
    assert!(engine.resolve_family("url(secret)").is_err());
    assert!(engine.families.borrow().collection.is_none());
    let first = engine
        .resolve_family("MissingOtterFamily, Consolas, Segoe UI")
        .unwrap();
    assert_eq!(&*first, "Consolas");
    assert!(Rc::ptr_eq(
        &first,
        &engine
            .resolve_family("MissingOtterFamily, Consolas, Segoe UI")
            .unwrap()
    ));
    assert_eq!(
        &*engine.resolve_family("Segoe UI, Consolas").unwrap(),
        "Segoe UI"
    );
    assert_eq!(&*engine.resolve_family("monospace").unwrap(), "Consolas");
    assert_eq!(&*engine.resolve_family("serif").unwrap(), "Times New Roman");
    assert_eq!(&*engine.resolve_family("'monospace'").unwrap(), "Segoe UI");
    assert_eq!(&*engine.resolve_family("OtterMissing").unwrap(), "Segoe UI");
    for i in 0..140 {
        let value = engine
            .resolve_family(&format!("OtterMissing{i}, sans-serif"))
            .unwrap();
        assert_eq!(&*value, "Segoe UI");
        assert!(engine.families.borrow().selected.len() <= 64);
    }
    assert_eq!(&*first, "Consolas"); // 清缓存不使旧场景持有的 Rc 悬垂。
}

#[test]
fn font_family_style_inheritance_layout_and_native_fields_share_selection() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","style":{"fontFamily":"MissingOtterFamily, monospace"},"children":[{"id":"text","type":"text","props":{"text":"WWii 0123"}},{"id":"field","type":"input"},{"id":"override","type":"text","props":{"text":"WWii 0123"},"style":{"fontFamily":"Segoe UI"}}]}"#).unwrap();
    let context = surface_viewport::context([400., 200.]);
    let engine = Engine::new().unwrap();
    let styles = Styles::build(&scene, surface_viewport::style(), context, &engine).unwrap();
    assert_eq!(styles.get(scene.root()).unwrap().family, "Consolas");
    let text = &scene.root().children()[0];
    assert_eq!(styles.get(text).unwrap().family, "Consolas");
    assert_eq!(
        styles.get(&scene.root().children()[2]).unwrap().family,
        "Segoe UI"
    );
    let layout = styles.layout_text(&engine, text, 400., 200.).unwrap();
    let expected = engine
        .layout(
            "WWii 0123",
            crate::surface_text::Style {
                family: "Consolas",
                ..surface_viewport::style()
            },
            400.,
            200.,
        )
        .unwrap();
    assert_eq!(layout.size(), expected.size());
    let presentation = crate::surface_presentation::Presentation::new(
        scene,
        surface_viewport::style(),
        context,
        [400., 200.],
    )
    .unwrap();
    assert_eq!(
        presentation.control_font("field").unwrap().family,
        "Consolas"
    );
}

#[test]
fn font_family_layout_node_and_replacement_scene_do_not_reuse_old_selection() {
    let parse_scene = |family: &str| {
        Scene::parse(&serde_json::to_vec(&serde_json::json!({"id":"same","type":"text","style":{"fontFamily":family},"props":{"text":"WWii 0123"}})).unwrap()).unwrap()
    };
    let original = parse_scene("Consolas");
    let next = parse_scene("Segoe UI");
    let viewport = surface_viewport::Viewport::new([400, 200], 96).unwrap();
    let engine = Engine::new().unwrap();
    let styles = Styles::build(
        &original,
        surface_viewport::style(),
        viewport.context(),
        &engine,
    )
    .unwrap();
    assert!(styles.get(next.root()).is_none());
    let a = engine
        .layout_node(
            original.root(),
            surface_viewport::style(),
            viewport.context(),
            400.,
            200.,
        )
        .unwrap();
    let b = engine
        .layout_node(
            next.root(),
            surface_viewport::style(),
            viewport.context(),
            400.,
            200.,
        )
        .unwrap();
    assert_ne!(a.size()[0], b.size()[0]);
    let invalid = parse_scene("url(secret)");
    assert!(Styles::build(
        &invalid,
        surface_viewport::style(),
        viewport.context(),
        &engine
    )
    .is_err());
    assert!(engine
        .layout_node(
            invalid.root(),
            surface_viewport::style(),
            viewport.context(),
            400.,
            200.
        )
        .is_err());
}
