use super::*;
use crate::surface_scene::Scene;
#[test]
fn stack_and_absolute_geometry_receive_child_font_context() {
    let scene=Scene::parse(br#"{"id":"root","type":"stack","layout":{"position":"relative","padding":"1em"},"children":[{"id":"a","type":"text","layout":{"width":"2em","height":"1em"}},{"id":"b","type":"text","layout":{"position":"absolute","left":"1em","top":"1em","width":"2em","height":"1em"}}]}"#).unwrap();
    let measured = [
        Measured {
            id: "a",
            size: [10., 10.],
            baseline: None,
        },
        Measured {
            id: "b",
            size: [10., 10.],
            baseline: None,
        },
    ];
    let fonts =
        |node: &crate::surface_scene::Node| Some(if node.id() == "root" { 10. } else { 20. });
    let result =
        arrange_with_fonts(scene.root(), &measured, [100., 100.], context(), &fonts).unwrap();
    assert_eq!(result[0].origin, [10., 10.]);
    assert_eq!(result[0].size, [40., 20.]);
    assert_eq!(result[1].origin, [20., 20.]);
    assert_eq!(result[1].size, [40., 20.]);
    assert!(matches!(
        arrange_with_fonts(scene.root(), &measured, [100., 100.], context(), &|_| Some(
            f32::NAN
        )),
        Err(Error::Input)
    ));
}
#[test]
fn stack_overlaps_children_in_scene_order_without_flow_gap() {
    let scene=Scene::parse(br#"{"id":"root","type":"stack","layout":{"padding":10,"gap":90},"children":[{"id":"back","type":"view"},{"id":"front","type":"text","layout":{"width":30,"height":20}}]}"#).unwrap();
    let measured = [
        Measured {
            id: "back",
            size: [10., 10.],
            baseline: None,
        },
        Measured {
            id: "front",
            size: [30., 20.],
            baseline: None,
        },
    ];
    let result = arrange(scene.root(), &measured, [100., 80.], context()).unwrap();
    assert_eq!(result[0].node.id(), "back");
    assert_eq!(result[1].node.id(), "front");
    assert_eq!(result[0].origin, [10., 10.]);
    assert_eq!(result[1].origin, [10., 10.]);
    assert_eq!(result[0].size, [80., 60.]);
    assert_eq!(result[1].size, [30., 20.]);
}
#[test]
fn stack_combines_alignment_relative_offset_and_absolute_overlay() {
    let scene=Scene::parse(br#"{"id":"root","type":"stack","layout":{"position":"relative","align":"center"},"children":[{"id":"text","type":"text","layout":{"height":20,"position":"relative","left":5}},{"id":"overlay","type":"text","layout":{"position":"absolute","right":10,"bottom":5,"width":15,"height":10}}]}"#).unwrap();
    let measured = [
        Measured {
            id: "text",
            size: [30., 20.],
            baseline: None,
        },
        Measured {
            id: "overlay",
            size: [15., 10.],
            baseline: None,
        },
    ];
    let result = arrange(scene.root(), &measured, [100., 80.], context()).unwrap();
    assert_eq!(result[0].origin, [5., 30.]);
    assert_eq!(result[0].size, [100., 20.]);
    assert_eq!(result[1].origin, [75., 65.]);
}
#[test]
fn relative_moves_visual_box_without_changing_sibling_flow() {
    let scene=Scene::parse(br#"{"id":"root","type":"row","children":[{"id":"a","type":"text","layout":{"width":20,"position":"relative","left":10,"top":5}},{"id":"b","type":"text","layout":{"width":20}}]}"#).unwrap();
    let measured = [
        Measured {
            id: "a",
            size: [20., 10.],
            baseline: None,
        },
        Measured {
            id: "b",
            size: [20., 10.],
            baseline: None,
        },
    ];
    let result = arrange(scene.root(), &measured, [100., 50.], context()).unwrap();
    assert_eq!(result[0].origin, [10., 5.]);
    assert_eq!(result[1].origin, [20., 0.]);
}
#[test]
fn absolute_exits_flow_and_uses_containing_block_edges() {
    let scene=Scene::parse(br#"{"id":"root","type":"row","layout":{"position":"relative","gap":10},"children":[{"id":"a","type":"text","layout":{"width":20}},{"id":"overlay","type":"text","layout":{"position":"absolute","left":10,"right":20,"bottom":5,"height":15}},{"id":"b","type":"text","layout":{"width":20}}]}"#).unwrap();
    let measured = [
        Measured {
            id: "a",
            size: [20., 10.],
            baseline: None,
        },
        Measured {
            id: "overlay",
            size: [999., 999.],
            baseline: None,
        },
        Measured {
            id: "b",
            size: [20., 10.],
            baseline: None,
        },
    ];
    let result = arrange(scene.root(), &measured, [100., 50.], context()).unwrap();
    assert_eq!(result[1].origin, [10., 30.]);
    assert_eq!(result[1].size, [70., 15.]);
    assert_eq!(result[2].origin, [30., 0.]);
}
#[test]
fn container_units_keep_host_query_container_and_bad_measurements_fail() {
    let scene=Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"a","type":"text","layout":{"width":"10cqw"}}]}"#).unwrap();
    let mut c = context();
    c.container = [400., 300.];
    let measured = [Measured {
        id: "a",
        size: [10., 10.],
        baseline: None,
    }];
    assert_eq!(
        arrange(scene.root(), &measured, [100., 100.], c).unwrap()[0].size[0],
        40.
    );
    let invalid = [Measured {
        id: "a",
        size: [f32::NAN, 10.],
        baseline: None,
    }];
    assert!(matches!(
        arrange(scene.root(), &invalid, [100., 100.], c),
        Err(Error::Input)
    ));
}
fn context() -> Context {
    Context {
        axis: 0.,
        viewport: [1000., 800.],
        container: [0.; 2],
        root_font: 16.,
        font: 16.,
        fraction: None,
    }
}
#[test]
fn actual_row_nodes_combine_padding_gap_growth_and_cross_alignment() {
    let scene=Scene::parse(br#"{"id":"root","type":"row","layout":{"padding":10,"gap":10,"align":"center"},"children":[{"id":"a","type":"text","layout":{"width":30,"height":20,"grow":1}},{"id":"b","type":"text","layout":{"width":50}}]}"#).unwrap();
    let measured = [
        Measured {
            id: "a",
            size: [30., 20.],
            baseline: None,
        },
        Measured {
            id: "b",
            size: [50., 10.],
            baseline: None,
        },
    ];
    let result = arrange(scene.root(), &measured, [200., 100.], context()).unwrap();
    assert_eq!(result[0].node.id(), "a");
    assert_eq!(result[0].origin, [10., 40.]);
    assert_eq!(result[0].size, [120., 20.]);
    assert_eq!(result[1].origin, [140., 45.]);
    assert_eq!(result[1].size, [50., 10.]);
}
#[test]
fn column_percentages_use_content_axis_and_explicit_width_overrides_stretch() {
    let scene=Scene::parse(br#"{"id":"root","type":"column","layout":{"padding":10},"children":[{"id":"a","type":"text","layout":{"height":"50%","width":40}}]}"#).unwrap();
    let result = arrange(
        scene.root(),
        &[Measured {
            id: "a",
            size: [10., 10.],
            baseline: None,
        }],
        [200., 100.],
        context(),
    )
    .unwrap();
    assert_eq!(result[0].origin, [10., 10.]);
    assert_eq!(result[0].size, [40., 40.]);
}
#[test]
fn stale_measurements_and_unsupported_position_are_not_silently_used() {
    let scene =
        Scene::parse(br#"{"id":"root","type":"row","children":[{"id":"a","type":"text"}]}"#)
            .unwrap();
    assert!(matches!(
        arrange(scene.root(), &[], [100., 100.], context()),
        Err(Error::Identity)
    ));
    assert!(matches!(
        arrange(
            scene.root(),
            &[Measured {
                id: "old",
                size: [10., 10.],
                baseline: None
            }],
            [100., 100.],
            context()
        ),
        Err(Error::Identity)
    ));
    let scene=Scene::parse(br#"{"id":"root","type":"row","children":[{"id":"a","type":"text","layout":{"position":"absolute"}}]}"#).unwrap();
    assert!(matches!(
        arrange(
            scene.root(),
            &[Measured {
                id: "a",
                size: [10., 10.],
                baseline: None
            }],
            [100., 100.],
            context()
        ),
        Err(Error::Unsupported)
    ));
}
