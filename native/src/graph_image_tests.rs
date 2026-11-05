//! 来源解析的基线合同；URI 只是借用标识，不访问网络、磁盘或用户图像。
use super::*;
#[test]
fn wrapped_formal_output_flows_through_connected_and_execution_paths() {
    let ids = [id("art"), id("target")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].kind = Kind::Art;
    nodes[0].preview = Some("not-formal");
    let fields = [
        ("value", Output::Null),
        ("src", Output::Image("formal-wrapped")),
    ];
    let values = [("output_image", Output::Object(&fields))];
    nodes[0].values = &values;
    let links = [link("edge", &ids[0], &ids[1], "image")];
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(
        graph.output(&ids[0], "output_image"),
        Some(Output::Object(&fields))
    );
    assert_eq!(
        graph.connected_image(&ids[1], "image"),
        Some("formal-wrapped")
    );
    assert_eq!(graph.execution_image(&ids[1]), Some("formal-wrapped"));
    assert_eq!(
        graph.execution_images(&ids[1], &[]).unwrap(),
        [("image", "formal-wrapped")]
    );
    let blocked = [
        ("value", Output::Boolean(false)),
        ("src", Output::Image("must-not-win")),
    ];
    let blocked_values = [
        ("output_image", Output::Object(&blocked)),
        ("output", Output::Image("must-not-fallback")),
    ];
    nodes[0].values = &blocked_values;
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(graph.execution_image(&ids[1]), None);
    assert_eq!(graph.image(&ids[1]), Some("local"));
}
#[test]
fn graph_rejects_ambiguous_output_keys_before_resolution() {
    let unit = id("a");
    let mut nodes = [sticker(&unit)];
    nodes[0].values = &[
        ("output", Output::Image("first")),
        ("output", Output::Image("second")),
    ];
    assert!(matches!(Graph::new(&nodes, &[]), Err(Error::InvalidOutput)));
    nodes[0].values = &[(
        "output",
        Output::Object(&[("src", Output::Null), ("src", Output::Image("ambiguous"))]),
    )];
    assert!(matches!(Graph::new(&nodes, &[]), Err(Error::InvalidOutput)));
}
#[test]
fn full_chain_borrows_source_and_rejects_unbounded_metadata() {
    let ids: Vec<_> = (0..MAX_UNITS).map(|index| id(&index.to_string())).collect();
    let source = String::from("source-owned-by-document");
    let mut nodes: Vec<_> = ids.iter().map(sticker).collect();
    nodes[0].source = Some(&source);
    let links: Vec<_> = (1..MAX_UNITS)
        .map(|index| {
            link(
                &format!("edge-{index}"),
                &ids[index - 1],
                &ids[index],
                "image",
            )
        })
        .collect();
    let image = Graph::new(&nodes, &links)
        .unwrap()
        .image(ids.last().unwrap())
        .unwrap();
    assert_eq!(image.as_ptr(), source.as_ptr());
    let excess = vec![
        Port {
            name: "image",
            kind: "image"
        };
        MAX_PORTS + 1
    ];
    nodes[0].inputs = &excess;
    assert!(matches!(Graph::new(&nodes, &links), Err(Error::Capacity)));
    nodes[0].inputs = &[];
    nodes.push(sticker(&ids[0]));
    assert!(matches!(Graph::new(&nodes, &links), Err(Error::Capacity)));
}
pub(super) fn id(value: &str) -> UnitId {
    UnitId::parse(value).unwrap()
}
pub(super) fn sticker(id: &UnitId) -> Node<'_> {
    Node {
        id,
        kind: Kind::Sticker,
        inputs: &[],
        outputs: &[],
        capability_inputs: None,
        capability_outputs: &[],
        values: &[],
        source: Some("local"),
        preview: None,
        manual_image: None,
        input_disabled: false,
        accept_upstream: true,
        locally_edited: false,
        pending: false,
    }
}
pub(super) fn link(name: &str, from: &UnitId, to: &UnitId, port: &str) -> Link {
    Link {
        id: id(name),
        from_unit: from.clone(),
        from_port: "output_image".into(),
        to_unit: to.clone(),
        to_port: port.into(),
    }
}
#[test]
fn upstream_chain_respects_all_local_ownership_boundaries() {
    let ids = [id("a"), id("b"), id("c")];
    let mut nodes = ids.iter().map(sticker).collect::<Vec<_>>();
    nodes[0].source = Some("source-a");
    let links = [
        link("ab", &ids[0], &ids[1], "image"),
        link("bc", &ids[1], &ids[2], "image"),
    ];
    assert_eq!(
        Graph::new(&nodes, &links).unwrap().image(&ids[2]),
        Some("source-a")
    );
    for boundary in 0..3 {
        nodes[1].input_disabled = boundary == 0;
        nodes[1].accept_upstream = boundary != 1;
        nodes[1].locally_edited = boundary == 2;
        assert_eq!(
            Graph::new(&nodes, &links).unwrap().image(&ids[2]),
            Some("local")
        );
    }
}
#[test]
fn art_preview_never_becomes_formal_output_and_pending_relays() {
    let ids = [id("art"), id("sticker")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].kind = Kind::Art;
    nodes[0].preview = Some("fast-preview");
    nodes[0].pending = true;
    let links = [link("ab", &ids[0], &ids[1], "image")];
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(graph.image(&ids[0]), Some("fast-preview"));
    assert_eq!(graph.output(&ids[0], "output_image"), None);
    assert_eq!(graph.image(&ids[1]), Some("local"));
    assert!(graph.formal_pending(&ids[1]));
    nodes[1].locally_edited = true;
    assert!(!Graph::new(&nodes, &links).unwrap().formal_pending(&ids[1]));
    nodes[1].locally_edited = false;
    nodes[0].values = &[("output", Output::Image("formal"))];
    assert_eq!(
        Graph::new(&nodes, &links).unwrap().image(&ids[1]),
        Some("formal")
    );
    nodes[0].values = &[
        ("output_image", Output::Other),
        ("output", Output::Image("not-fallback")),
    ];
    assert_eq!(
        Graph::new(&nodes, &links).unwrap().image(&ids[1]),
        Some("local")
    );
}
#[test]
fn manual_data_fallback_requires_relay_and_ignores_file_path() {
    let id = id("a");
    let mut nodes = [sticker(&id)];
    nodes[0].manual_image = Some("data:image/png;base64,fixture");
    assert_eq!(
        Graph::new(&nodes, &[]).unwrap().image(&id),
        nodes[0].manual_image
    );
    nodes[0].input_disabled = true;
    assert_eq!(Graph::new(&nodes, &[]).unwrap().image(&id), Some("local"));
    nodes[0].input_disabled = false;
    nodes[0].manual_image = Some("C:/not-read.png");
    nodes[0].preview = Some("preview");
    assert_eq!(Graph::new(&nodes, &[]).unwrap().image(&id), Some("preview"));
}
#[test]
fn capability_empty_inputs_override_unit_ports_and_port_order_follows_links() {
    let ids = [id("a"), id("b"), id("c")];
    let mut nodes = ids.iter().map(sticker).collect::<Vec<_>>();
    nodes[0].source = Some("first");
    nodes[1].source = Some("second");
    nodes[2].kind = Kind::Art;
    nodes[2].inputs = &[
        Port {
            name: "source_file",
            kind: "string",
        },
        Port {
            name: "mask",
            kind: "IMAGE",
        },
    ];
    let links = [
        link("ac", &ids[0], &ids[2], "mask"),
        link("bc", &ids[1], &ids[2], "source_file"),
    ];
    assert_eq!(
        Graph::new(&nodes, &links).unwrap().image(&ids[2]),
        Some("first")
    );
    nodes[2].capability_inputs = Some(&[]);
    assert_eq!(
        Graph::new(&nodes, &links).unwrap().image(&ids[2]),
        Some("local")
    );
    nodes[2].capability_inputs = Some(&[Port {
        name: "source_file",
        kind: "string",
    }]);
    assert_eq!(
        Graph::new(&nodes, &links).unwrap().image(&ids[2]),
        Some("second")
    );
}
#[test]
fn imported_cycles_missing_nodes_and_non_image_outputs_are_bounded() {
    let ids = [id("a"), id("b")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    let links = [
        link("ab", &ids[0], &ids[1], "image"),
        link("ba", &ids[1], &ids[0], "image"),
    ];
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(graph.image(&ids[0]), Some("local"));
    assert!(!graph.formal_pending(&ids[0]));
    assert_eq!(graph.image(&id("missing")), None);
    assert_eq!(graph.output(&ids[0], "number"), None);
    nodes[0].outputs = &[Port {
        name: "custom",
        kind: "image/png",
    }];
    assert_eq!(
        Graph::new(&nodes, &links)
            .unwrap()
            .output(&ids[0], "custom"),
        Some(Output::Image("local"))
    );
    let duplicated = [sticker(&ids[0]), sticker(&ids[0])];
    assert!(matches!(
        Graph::new(&duplicated, &[]),
        Err(Error::DuplicateId)
    ));
}
