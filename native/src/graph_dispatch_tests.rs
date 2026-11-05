//! 正式执行候选的组合回归；不注册请求、不执行节点、不发送网络消息。
use super::super::tests::{id, link, sticker};
use super::*;
use crate::graph_param_coercion::Widget;
fn options(trigger: Trigger) -> DispatchOptions {
    DispatchOptions {
        config: Config::default(),
        runtime: Runtime::default(),
        trigger,
    }
}
#[test]
fn automatic_missing_images_skip_but_manual_can_prepare() {
    let unit = id("art");
    let mut nodes = [sticker(&unit)];
    nodes[0].kind = Kind::Art;
    nodes[0].capability_inputs = Some(&[Port {
        name: "image",
        kind: "image",
    }]);
    let graph = Graph::new(&nodes, &[]).unwrap();
    let result = graph
        .prepare_dispatch(&unit, &[], &[], options(Trigger::Upstream))
        .unwrap();
    assert!(
        matches!(result, Preparation::Skipped(SkipReason::MissingImages(ref ports)) if ports == &["image"])
    );
    assert!(matches!(
        graph
            .prepare_dispatch(&unit, &[], &[], options(Trigger::Manual))
            .unwrap(),
        Preparation::Ready(_)
    ));
}
#[test]
fn prepared_candidate_combines_formal_images_coercion_and_disabled_filtering() {
    let ids = [id("source"), id("art")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].source = Some("source-image");
    nodes[0].values = &[("amount", Output::Image("12"))];
    nodes[1].kind = Kind::Art;
    nodes[1].capability_inputs = Some(&[Port {
        name: "image",
        kind: "image",
    }]);
    let mut edges = [
        link("image", &ids[0], &ids[1], "image"),
        link("amount", &ids[0], &ids[1], "amount"),
    ];
    edges[1].from_port = "amount".into();
    let declarations = [
        Param {
            id: "amount",
            default: Some(Output::Number(1.0)),
            secret: false,
            widget: Widget {
                name: "number",
                min: None,
                max: Some(10.0),
            },
        },
        Param {
            id: "secret",
            default: None,
            secret: true,
            widget: Widget::default(),
        },
    ];
    let manual = [
        ("disabled", Output::Image(DISABLED)),
        ("secret", Output::Image(DISABLED)),
        ("__exec_mode", Output::Image(DISABLED)),
    ];
    let graph = Graph::new(&nodes, &edges).unwrap();
    let Preparation::Ready(result) = graph
        .prepare_dispatch(
            &ids[1],
            &declarations,
            &manual,
            options(Trigger::Parameter { final_change: true }),
        )
        .unwrap()
    else {
        panic!("Expected complete candidate");
    };
    assert_eq!(result.images, [("image", "source-image")]);
    assert_eq!(result.disabled, ["disabled"]);
    assert_eq!(
        result.active_params().collect::<Vec<_>>(),
        [("amount", Some(Output::Number(10.0)))]
    );
}
#[test]
fn nonfinal_preview_only_and_missing_targets_never_prepare() {
    let ids = [id("art"), id("sticker")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].kind = Kind::Art;
    let graph = Graph::new(&nodes, &[]).unwrap();
    assert!(matches!(
        graph
            .prepare_dispatch(
                &ids[0],
                &[],
                &[],
                options(Trigger::Parameter {
                    final_change: false
                })
            )
            .unwrap(),
        Preparation::Skipped(SkipReason::Gate(Skip::NonFinalParameter))
    ));
    let mut shader = options(Trigger::Manual);
    shader.runtime.shader = true;
    assert!(matches!(
        graph.prepare_dispatch(&ids[0], &[], &[], shader).unwrap(),
        Preparation::Skipped(SkipReason::Gate(Skip::ShaderOnly))
    ));
    assert!(matches!(
        graph
            .prepare_dispatch(&ids[1], &[], &[], options(Trigger::Manual))
            .unwrap(),
        Preparation::Skipped(SkipReason::NotArt)
    ));
    assert!(matches!(
        graph
            .prepare_dispatch(&id("missing"), &[], &[], options(Trigger::Manual))
            .unwrap(),
        Preparation::Skipped(SkipReason::MissingTarget)
    ));
}
#[test]
fn malformed_params_fail_without_mutating_graph_outputs() {
    let unit = id("art");
    let mut nodes = [sticker(&unit)];
    nodes[0].kind = Kind::Art;
    nodes[0].values = &[("output", Output::Image("formal"))];
    let graph = Graph::new(&nodes, &[]).unwrap();
    let manual = [
        ("duplicate", Output::Null),
        ("duplicate", Output::Boolean(true)),
    ];
    assert!(matches!(
        graph.prepare_dispatch(&unit, &[], &manual, options(Trigger::Manual)),
        Err(Error::InvalidOutput)
    ));
    assert_eq!(graph.output(&unit, "output"), Some(Output::Image("formal")));
}
