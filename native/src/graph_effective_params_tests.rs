//! 图输出、包装值、widget 和安全过滤的组合回归，不调用外部执行服务。
use super::super::tests::{id, link, sticker};
use super::*;
use crate::{graph_param_coercion::Widget, graph_params::DISABLED};
fn declaration<'a>(id: &'a str, widget: &'a str) -> Param<'a> {
    Param {
        id,
        default: Some(Output::Number(3.0)),
        secret: false,
        widget: Widget {
            name: widget,
            min: None,
            max: None,
        },
    }
}
#[test]
fn wrapped_output_is_coerced_clamped_and_does_not_mutate_manual() {
    let ids = [id("source"), id("target")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].kind = Kind::Art;
    nodes[1].kind = Kind::Art;
    nodes[0].values = &[(
        "output_image",
        Output::Object(&[("value", Output::Image("12.5"))]),
    )];
    let edges = [link("edge", &ids[0], &ids[1], "amount")];
    let graph = Graph::new(&nodes, &edges).unwrap();
    let mut declarations = [declaration("amount", "number")];
    declarations[0].widget.max = Some(10.0);
    let manual = [("amount", Output::Number(2.0))];
    let result = graph
        .effective_params(&ids[1], &declarations, &manual)
        .unwrap();
    assert_eq!(
        result.entries().collect::<Vec<_>>(),
        [("amount", Some(Output::Number(10.0)))]
    );
    assert_eq!(manual[0].1, Output::Number(2.0));
    assert_eq!(
        graph.output(&ids[0], "output_image"),
        Some(Output::Object(&[("value", Output::Image("12.5"))]))
    );
}
#[test]
fn invalid_later_link_preserves_earlier_value_and_valid_later_link_wins() {
    let ids = [id("first"), id("second"), id("target")];
    let mut nodes = ids.iter().map(sticker).collect::<Vec<_>>();
    for node in &mut nodes {
        node.kind = Kind::Art;
    }
    nodes[0].values = &[("output_image", Output::Number(8.0))];
    nodes[1].values = &[("output_image", Output::Image("bad-number"))];
    let edges = [
        link("first", &ids[0], &ids[2], "amount"),
        link("second", &ids[1], &ids[2], "amount"),
    ];
    let declared = [declaration("amount", "number")];
    assert_eq!(
        Graph::new(&nodes, &edges)
            .unwrap()
            .effective_params(&ids[2], &declared, &[])
            .unwrap()
            .entries()
            .next(),
        Some(("amount", Some(Output::Number(8.0))))
    );
    nodes[1].values = &[("output_image", Output::Number(9.0))];
    assert_eq!(
        Graph::new(&nodes, &edges)
            .unwrap()
            .effective_params(&ids[2], &declared, &[])
            .unwrap()
            .entries()
            .next(),
        Some(("amount", Some(Output::Number(9.0))))
    );
}
#[test]
fn secret_control_disabled_and_undeclared_ports_do_not_import_upstream() {
    let ids = [id("source"), id("target")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].values = &[("output_image", Output::Image("upstream"))];
    nodes[1].kind = Kind::Art;
    let mut declared = [
        declaration("secret", "text"),
        declaration("__exec_mode", "text"),
        declaration("disabled", "text"),
    ];
    declared[0].secret = true;
    let manual = [
        ("disabled", Output::Image(DISABLED)),
        ("unknown", Output::Image("manual")),
    ];
    let edges: Vec<_> = ["secret", "__exec_mode", "disabled", "unknown"]
        .into_iter()
        .map(|name| link(name, &ids[0], &ids[1], name))
        .collect();
    let result = Graph::new(&nodes, &edges)
        .unwrap()
        .effective_params(&ids[1], &declared, &manual)
        .unwrap();
    assert_eq!(
        result.entries().collect::<Vec<_>>(),
        [
            ("disabled", Some(Output::Image(DISABLED))),
            ("unknown", Some(Output::Image("manual")))
        ]
    );
}
#[test]
fn absent_art_output_keeps_default_and_empty_source_port_uses_output_alias() {
    let ids = [id("source"), id("target")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].kind = Kind::Art;
    nodes[0].preview = Some("not-formal");
    nodes[1].kind = Kind::Art;
    let mut edges = [link("edge", &ids[0], &ids[1], "label")];
    edges[0].from_port.clear();
    let declared = [declaration("label", "text")];
    assert_eq!(
        Graph::new(&nodes, &edges)
            .unwrap()
            .effective_params(&ids[1], &declared, &[])
            .unwrap()
            .entries()
            .next(),
        Some(("label", Some(Output::Number(3.0))))
    );
    nodes[0].values = &[("output", Output::Number(1e21))];
    let graph = Graph::new(&nodes, &edges).unwrap();
    let result = graph.effective_params(&ids[1], &declared, &[]).unwrap();
    assert_eq!(
        result.entries().next(),
        Some(("label", Some(Output::Image("1e+21"))))
    );
}
#[test]
fn absent_target_is_empty_and_non_art_keeps_only_public_manual_params() {
    let unit = id("sticker");
    let nodes = [sticker(&unit)];
    let graph = Graph::new(&nodes, &[]).unwrap();
    let declared = [declaration("art-only", "text")];
    let manual = [
        ("own", Output::Number(7.0)),
        ("force_update", Output::Boolean(true)),
    ];
    assert_eq!(
        graph
            .effective_params(&unit, &declared, &manual)
            .unwrap()
            .entries()
            .collect::<Vec<_>>(),
        [("own", Some(Output::Number(7.0)))]
    );
    assert_eq!(
        graph
            .effective_params(&id("missing"), &declared, &manual)
            .unwrap()
            .entries()
            .len(),
        0
    );
}
