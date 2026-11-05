//! 四边框必须改变同一布局几何，不允许只保存 CSS 或绘制线条而不占内容空间。
use super::*;
#[test]
fn asymmetric_borders_inset_flow_and_absolute_padding_box_independently() {
    let scene = Scene::parse(br#"{"id":"root","type":"column","layout":{"position":"relative","width":100,"height":80,"paddingTop":3,"paddingRight":5,"paddingBottom":7,"paddingLeft":11},"style":{"borderTopWidth":2,"borderRightWidth":4,"borderBottomWidth":6,"borderLeftWidth":8},"children":[{"id":"flow","type":"view","layout":{"width":20,"height":10}},{"id":"absolute","type":"view","layout":{"position":"absolute","top":0,"right":0,"bottom":0,"left":0}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 80.]).unwrap();
    let tree = Tree::build(&scene, [100., 80.], context(), &mut measure).unwrap();
    assert_eq!(
        tree.nodes()[1].origin,
        [19., 5.],
        "each border side must consume its own space"
    );
    assert_eq!(tree.nodes()[1].size, [20., 10.]);
    assert_eq!(tree.nodes()[2].origin, [8., 2.]);
    assert_eq!(tree.nodes()[2].size, [88., 72.]);
}
#[test]
fn border_sides_hit_testing_uses_same_asymmetric_inner_corner_as_paint() {
    let scene=Scene::parse(br#"{"id":"root","type":"view","layout":{"position":"relative","height":64,"overflowX":"hidden","overflowY":"hidden"},"style":{"borderTopWidth":2,"borderRightWidth":6,"borderBottomWidth":12,"borderLeftWidth":18,"borderRadius":24},"children":[{"id":"child","type":"view","layout":{"position":"absolute","top":0,"right":0,"bottom":0,"left":0}}]}"#).unwrap();
    let view = crate::surface_presentation::Presentation::new(scene, base(), context(), [64., 64.])
        .unwrap();
    assert_eq!(view.hit_test([19., 3.]), Some("root"));
    assert_eq!(view.hit_test([25., 10.]), Some("child"));
    assert_eq!(view.hit_test([56., 50.]), Some("root"));
    assert_eq!(view.hit_test([1., 1.]), None);
}
#[test]
fn border_sides_font_relative_widths_feed_grid_flex_stack_and_intrinsic_content() {
    for (kind, layout) in [
        ("column", serde_json::json!({})),
        ("row", serde_json::json!({})),
        ("stack", serde_json::json!({})),
        (
            "grid",
            serde_json::json!({"gridTemplateColumns":"minmax(0,1fr)"}),
        ),
    ] {
        let mut value = serde_json::json!({"id":"root","type":kind,"layout":layout,
            "style":{"fontSize":"2em","borderTopWidth":"0.25em","borderRightWidth":"0.5em",
            "borderBottomWidth":"0.75em","borderLeftWidth":"1em"},
            "children":[{"id":"child","type":"view","layout":{"width":20,"height":10}}]});
        value["layout"]["width"] = 100.into();
        value["layout"]["height"] = 80.into();
        let scene = Scene::parse(value.to_string().as_bytes()).unwrap();
        let mut measure = NativeMeasure::new(&scene, base(), context(), [100., 80.]).unwrap();
        let prepared = measure.prepare(&scene, [100., 80.], context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].origin, [32., 8.], "{kind}");
        assert_eq!(prepared.tree().nodes()[1].size, [20., 10.], "{kind}");
    }
    let scene=Scene::parse(br#"{"id":"root","type":"column","children":[{"id":"text","type":"text","props":{"text":"Native"},"style":{"whiteSpace":"nowrap","borderTopWidth":2,"borderRightWidth":4,"borderBottomWidth":6,"borderLeftWidth":8}}]}"#).unwrap();
    let mut measure = NativeMeasure::new(&scene, base(), context(), [400., 300.]).unwrap();
    let prepared = measure.prepare(&scene, [400., 300.], context()).unwrap();
    let text = &prepared.text().items()[0];
    assert_eq!(text.origin(), [8., 2.]);
    assert!((prepared.tree().nodes()[1].size[1] - text.layout().size()[1] - 8.).abs() < 0.01);
}
