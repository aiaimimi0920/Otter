//! 对照主干执行输入判定：显示回退不等于执行就绪，辅助输入只来自能力声明。
use super::super::tests::{id, link, sticker};
use super::*;
#[test]
fn connected_pending_art_blocks_execution_but_not_local_display() {
    let ids = [id("art"), id("sticker")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[0].kind = Kind::Art;
    nodes[0].preview = Some("preview");
    let links = [link("edge", &ids[0], &ids[1], "image")];
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(graph.image(&ids[1]), Some("local"));
    assert_eq!(graph.execution_image(&ids[1]), None);
    assert_eq!(graph.execution_image(&ids[0]), None);
    assert_eq!(
        Graph::new(&nodes, &[]).unwrap().execution_image(&ids[1]),
        Some("local")
    );
}
#[test]
fn connection_wins_over_manual_and_auxiliary_is_capability_declared() {
    let ids = [id("source"), id("art")];
    let mut nodes = [sticker(&ids[0]), sticker(&ids[1])];
    nodes[1].kind = Kind::Art;
    let ports = [
        Port {
            name: "image",
            kind: "image",
        },
        Port {
            name: "mask",
            kind: "image",
        },
        Port {
            name: "strength",
            kind: "number",
        },
    ];
    nodes[1].capability_inputs = Some(&ports);
    let links = [link("edge", &ids[0], &ids[1], "image")];
    let manual = [
        ("image", Output::Image("manual-primary")),
        ("mask", Output::Image("manual-mask")),
        ("strength", Output::Image("not-image-port")),
    ];
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(
        graph.execution_images(&ids[1], &manual).unwrap(),
        [("image", "local"), ("mask", "manual-mask")]
    );
    assert_eq!(
        graph.auxiliary_execution_images(&ids[1], &manual).unwrap(),
        [("mask", "manual-mask")]
    );
    assert!(graph
        .missing_execution_images(&ids[1], &manual)
        .unwrap()
        .is_empty());
    assert_eq!(
        graph.missing_execution_images(&ids[1], &[]).unwrap(),
        ["mask"]
    );
    nodes[1].inputs = &ports;
    nodes[1].capability_inputs = None;
    let graph = Graph::new(&nodes, &links).unwrap();
    assert_eq!(graph.execution_images(&ids[1], &manual).unwrap().len(), 2);
    assert!(graph
        .auxiliary_execution_images(&ids[1], &manual)
        .unwrap()
        .is_empty());
    assert_eq!(
        graph.missing_execution_images(&ids[1], &manual).unwrap(),
        ["mask"]
    );
}
#[test]
fn explicit_empty_capability_inputs_do_not_inherit_or_invent_ports() {
    let unit = id("art");
    let mut nodes = [sticker(&unit)];
    nodes[0].kind = Kind::Art;
    nodes[0].inputs = &[Port {
        name: "image",
        kind: "image",
    }];
    nodes[0].capability_inputs = Some(&[]);
    let manual = [("image", Output::Image("manual"))];
    let graph = Graph::new(&nodes, &[]).unwrap();
    assert!(graph.execution_images(&unit, &manual).unwrap().is_empty());
    assert!(graph
        .missing_execution_images(&unit, &manual)
        .unwrap()
        .is_empty());
}
#[test]
fn missing_primary_does_not_use_manual_and_deduplicates_declared_ports() {
    let unit = id("art");
    let mut nodes = [sticker(&unit)];
    nodes[0].kind = Kind::Art;
    nodes[0].capability_inputs = Some(&[
        Port {
            name: "mask",
            kind: "image",
        },
        Port {
            name: "IMAGE",
            kind: "image",
        },
        Port {
            name: "mask",
            kind: "image",
        },
        Port {
            name: "__proto__",
            kind: "image",
        },
    ]);
    let manual = [
        ("IMAGE", Output::Image("manual-main")),
        ("mask", Output::Image("manual-mask")),
        ("__proto__", Output::Image("safe-field")),
    ];
    let graph = Graph::new(&nodes, &[]).unwrap();
    // 主干 missing 判定的主输入只查连接，手动值只补充辅助端口，不能悄悄改变这一语义。
    assert_eq!(
        graph.missing_execution_images(&unit, &manual).unwrap(),
        ["IMAGE"]
    );
    assert_eq!(
        graph.execution_images(&unit, &manual).unwrap(),
        [
            ("mask", "manual-mask"),
            ("IMAGE", "manual-main"),
            ("__proto__", "safe-field")
        ]
    );
}
#[test]
fn default_sticker_input_manual_values_and_budgets_are_explicit() {
    let unit = id("sticker");
    let nodes = [sticker(&unit)];
    let graph = Graph::new(&nodes, &[]).unwrap();
    assert_eq!(
        graph
            .execution_images(&unit, &[("image", Output::Image("manual"))])
            .unwrap(),
        [("image", "manual")]
    );
    assert!(graph
        .execution_images(&unit, &[("image", Output::Other)])
        .unwrap()
        .is_empty());
    assert!(graph
        .execution_images(&unit, &[("image", Output::Image(""))])
        .unwrap()
        .is_empty());
    let excess = vec![("image", Output::Image("manual")); MAX_PORTS + 1];
    assert_eq!(graph.execution_images(&unit, &excess), Err(Error::Capacity));
    assert_eq!(
        graph.auxiliary_execution_images(&unit, &excess),
        Err(Error::Capacity)
    );
    assert_eq!(
        graph.missing_execution_images(&unit, &excess),
        Err(Error::Capacity)
    );
}
