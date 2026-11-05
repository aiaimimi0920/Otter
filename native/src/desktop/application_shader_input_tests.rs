//! Unit 字符串只提升当前 workspace 已拥有的图片；秘密/路径/预览不获得来源权。
use super::*;
use crate::{
    graph_image::{Kind, Node},
    graph_params::Param,
};
fn node(id: &UnitId) -> Node<'_> {
    Node {
        id,
        kind: Kind::Art,
        inputs: &[],
        outputs: &[],
        capability_inputs: Some(&[]),
        capability_outputs: &[],
        values: &[],
        source: None,
        preview: Some("display-only"),
        manual_image: None,
        input_disabled: false,
        accept_upstream: true,
        locally_edited: false,
        pending: false,
    }
}
#[test]
fn shader_unit_parameters_select_public_owned_ids_without_path_or_widget_guessing() {
    let unit = UnitId::parse("local-unit").unwrap();
    let workspace = Workspace::with_initial(unit.clone());
    let declared = [Param {
        id: "secret",
        default: None,
        secret: true,
        widget: Default::default(),
    }];
    let manual = [
        ("legacy", Output::Image("local-unit")),
        ("secret", Output::Image("local-unit")),
        ("reference", Output::Image("local-unit")),
        ("__ui", Output::Image("local-unit")),
        ("input", Output::Image("local-unit")),
        ("source", Output::Image("local-unit")),
        ("disabled", Output::Image(crate::graph_params::DISABLED)),
        ("text", Output::Image("unknown-unit")),
        ("url", Output::Image("https://example.invalid/image.png")),
        ("path", Output::Image("C:/not-read.png")),
        ("uri", Output::Image("data:image/png;base64,AA==")),
    ];
    let parameters = Params::new(&declared, &manual).unwrap();
    assert_eq!(
        references(&workspace, Some(&parameters), "source").unwrap(),
        [("legacy".into(), &unit)]
    );
    assert!(references(&workspace, None, "source").unwrap().is_empty());
    for names in [
        vec!["bad/path".to_string()],
        (0..32).map(|n| format!("image{n}")).collect(),
    ] {
        let values: Vec<_> = names
            .iter()
            .map(|name| (name.as_str(), Output::Image("local-unit")))
            .collect();
        assert!(references(
            &workspace,
            Some(&Params::new(&[], &values).unwrap()),
            "input"
        )
        .is_err());
    }
}
#[test]
fn shader_unit_parameters_require_formal_art_output_and_keep_prepared_connection_priority() {
    let unit = UnitId::parse("art").unwrap();
    let raw =
        serde_json::json!({"id":"shader", "metadata":{"capabilities":{"shaderInput":"source"}}});
    let cap = Capability::new(&raw).unwrap();
    let refs = [("overlay".into(), &unit)];
    let mut images = vec![("source", "primary")];
    let nodes = [node(&unit)];
    let graph = Graph::new(&nodes, &[]).unwrap();
    assert!(resolve(Some(&cap), &graph, &refs, &mut images).is_err());
    assert_eq!(images, [("source", "primary")]);
    let mut formal = node(&unit);
    let values = [("output", Output::Image("formal-png"))];
    formal.values = &values;
    let nodes = [formal];
    let graph = Graph::new(&nodes, &[]).unwrap();
    assert_eq!(
        resolve(Some(&cap), &graph, &refs, &mut images)
            .unwrap()
            .parameters,
        ["overlay"]
    );
    assert_eq!(images, [("source", "primary"), ("overlay", "formal-png")]);
    let mut connected = vec![("source", "primary"), ("overlay", "connected-png")];
    resolve(Some(&cap), &graph, &refs, &mut connected).unwrap();
    assert_eq!(connected[1], ("overlay", "connected-png"));
}
#[test]
fn shader_unit_prepared_bindings_require_same_public_value_source_and_union_limit() {
    use crate::desktop::shader_parameters::prepared_parameters;
    let raw = serde_json::json!({"id":"shader"});
    let cap = Capability::new(&raw).unwrap();
    let values = [
        ("legacy", Output::Image("local-unit")),
        ("disabled", Output::Image(crate::graph_params::DISABLED)),
    ];
    let parameters = Params::new(&[], &values).unwrap();
    assert_eq!(
        prepared_parameters(
            &cap,
            &parameters,
            "source",
            &["source", "legacy"],
            &["legacy".into()]
        )
        .unwrap(),
        ["legacy"]
    );
    for name in [
        "source",
        "input",
        "reference",
        "inputs",
        "params",
        "mode",
        "output_mode",
        "__ui",
        "disabled",
        "absent",
    ] {
        assert!(prepared_parameters(&cap, &parameters, "source", &[name], &[name.into()]).is_err());
    }
    let names: Vec<_> = (0..32).map(|n| format!("image{n}")).collect();
    let values: Vec<_> = names
        .iter()
        .map(|name| (name.as_str(), Output::Image("local-unit")))
        .collect();
    let ports: Vec<_> = names.iter().map(String::as_str).collect();
    assert!(prepared_parameters(
        &cap,
        &Params::new(&[], &values).unwrap(),
        "source",
        &ports,
        &names
    )
    .is_err());
}
