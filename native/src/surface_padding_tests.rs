//! 四边内缩跨真实 DirectWrite、内容流、Grid 和图片比例反馈，不复制布局算法。
use super::*;
use crate::surface_box::{self, Insets};
use serde_json::json;
fn parse(value: serde_json::Value) -> Scene {
    Scene::parse(&serde_json::to_vec(&value).unwrap()).unwrap()
}
fn sides() -> serde_json::Value {
    json!({"paddingTop":3,"paddingRight":7,"paddingBottom":11,"paddingLeft":13})
}
#[test]
fn padding_sides_override_legacy_scalar_and_preserve_control_defaults() {
    for (kind, defaults, border) in [
        ("view", [0., 0.], 0.),
        ("button", [12., 6.], 1.),
        ("textarea", [9., 6.], 1.),
    ] {
        let scene = parse(json!({"id":"root","type":kind,"layout":{"paddingLeft":20}}));
        let value = crate::surface_button::insets(scene.root(), 200., context());
        assert_eq!(
            value,
            Insets {
                left: 20. + border,
                top: defaults[1] + border,
                right: defaults[0] + border,
                bottom: defaults[1] + border
            }
        );
        let scene = parse(json!({"id":"root","type":kind,"layout":{
            "padding":4,"paddingLeft":12,"paddingRight":0,"paddingTop":1,"paddingBottom":3}}));
        let value = crate::surface_button::insets(scene.root(), 200., context());
        assert_eq!(value.start(), [12. + border, 1. + border]);
        assert_eq!(value.total(), [12. + 2. * border, 4. + 2. * border]);
    }
}
#[test]
fn padding_flow_stack_and_grid_use_start_edges_and_axis_totals() {
    for kind in ["row", "column", "stack", "grid"] {
        let mut layout = sides();
        layout["height"] = 100.into();
        let scene = parse(
            json!({"id":"root","type":kind,"layout":layout,"style":{"borderWidth":2},
            "children":[{"id":"child","type":"view","layout":{"width":20,"height":10}}]}),
        );
        let mut measure = NativeMeasure::new(&scene, base(), context(), [200., 100.]).unwrap();
        let prepared = measure.prepare(&scene, [200., 100.], context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].origin, [15., 5.], "{kind}");
        assert_eq!(prepared.tree().nodes()[1].size, [20., 10.], "{kind}");
        assert_eq!(prepared.tree().nodes()[1].containing_width, 176.);
    }
    for kind in ["row", "column", "stack", "grid"] {
        let scene =
            parse(json!({"id":"root","type":kind,"layout":sides(),"style":{"borderWidth":2}}));
        let mut measure = NativeMeasure::new(&scene, base(), context(), [200., 100.]).unwrap();
        assert_eq!(
            measure
                .measure(scene.root(), [200., 100.], context())
                .unwrap()
                .size,
            [24., 18.],
            "{kind}"
        );
    }
}
#[test]
fn padding_absolute_anchors_stay_at_padding_box_and_minima_are_per_axis() {
    for kind in ["row", "column", "stack"] {
        let mut layout = sides();
        layout["position"] = "relative".into();
        layout["height"] = 100.into();
        let scene = parse(
            json!({"id":"root","type":kind,"layout":layout,"style":{"borderWidth":2},
            "children":[{"id":"absolute","type":"view","layout":{"position":"absolute","left":0,"top":0,"right":0,"bottom":0}}]}),
        );
        let mut measure = NativeMeasure::new(&scene, base(), context(), [200., 100.]).unwrap();
        let prepared = measure.prepare(&scene, [200., 100.], context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].origin, [2., 2.]);
        assert_eq!(prepared.tree().nodes()[1].size, [196., 96.]);
        let mut small = sides();
        for key in ["width", "height", "maxWidth", "maxHeight"] {
            small[key] = 1.into();
        }
        let scene = parse(
            json!({"id":"root","type":kind,"children":[{"id":"child","type":"view","layout":small,"style":{"borderWidth":2}}]}),
        );
        let mut measure = NativeMeasure::new(&scene, base(), context(), [10., 10.]).unwrap();
        let prepared = measure.prepare(&scene, [10., 10.], context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].size, [24., 18.]);
    }
}
#[test]
fn padding_percentages_all_use_containing_width_and_em_uses_computed_font() {
    let scene = parse(
        json!({"id":"root","type":"row","layout":{"align":"flex-start"},"children":[{
        "id":"text","type":"text","props":{"text":"A wrapping line with several words"},
        "layout":{"width":100,"paddingLeft":"10%","paddingRight":"5%","paddingTop":"0.5em","paddingBottom":"1em"},
        "style":{"fontSize":20,"borderWidth":2}}]}),
    );
    let mut measure = NativeMeasure::new(&scene, base(), context(), [200., 300.]).unwrap();
    let prepared = measure.prepare(&scene, [200., 300.], context()).unwrap();
    let text = &prepared.text().items()[0];
    assert_eq!(text.origin(), [22., 12.]);
    assert_eq!(text.allocated()[0], 66.);
    let height = text.layout().size()[1] + 34.;
    assert!((prepared.tree().nodes()[1].size[1] - height).abs() < 0.01);
    assert!((text.baseline - text.layout().baseline() - 12.).abs() < 0.01);
    assert!(prepared.passes() <= 8);
}
#[test]
fn padding_button_uses_top_for_baseline_and_both_edges_for_centering() {
    let scene = parse(json!({"id":"root","type":"button","props":{"label":"Run"},
        "layout":{"width":120,"height":60,"paddingTop":3,"paddingBottom":11,"paddingLeft":20,"paddingRight":4}}));
    let mut measure = NativeMeasure::new(&scene, base(), context(), [200., 100.]).unwrap();
    let prepared = measure.prepare(&scene, [200., 100.], context()).unwrap();
    let text = &prepared.text().items()[0];
    assert_eq!(text.allocated(), [94., 44.]);
    assert_eq!(text.origin()[0], 21.);
    assert!((text.origin()[1] - (4. + (44. - text.layout().size()[1]).max(0.) / 2.)).abs() < 0.01);
}
#[test]
fn padding_image_ratio_uses_different_axis_totals_through_reflow() {
    for (kind, layout, expected) in [
        (
            "column",
            json!({"width":160,"paddingLeft":10,"paddingRight":30,"paddingTop":5,"paddingBottom":15}),
            [160., 80.],
        ),
        (
            "row",
            json!({"height":80,"paddingLeft":10,"paddingRight":30,"paddingTop":5,"paddingBottom":15}),
            [160., 80.],
        ),
        (
            "grid",
            json!({"width":160,"paddingLeft":10,"paddingRight":30,"paddingTop":5,"paddingBottom":15}),
            [160., 80.],
        ),
    ] {
        let scene = parse(
            json!({"id":"root","type":kind,"layout":{"align":"flex-start"},
            "children":[{"id":"image","type":"image","layout":layout}]}),
        );
        let mut measure = NativeMeasure::with_images(
            &scene,
            base(),
            context(),
            [300., 180.],
            [("image".into(), [80, 40])].into(),
        )
        .unwrap();
        let prepared = measure.prepare(&scene, [300., 180.], context()).unwrap();
        assert_eq!(prepared.tree().nodes()[1].size, expected, "{kind}");
    }
}
#[test]
fn padding_asymmetric_collapsed_content_still_has_noncompressible_box() {
    let scene = parse(
        json!({"id":"root","type":"text","props":{"text":"not drawn"},
        "layout":{"width":1,"height":1,"paddingLeft":30,"paddingTop":7,"paddingRight":10,"paddingBottom":13},"style":{"borderWidth":2}}),
    );
    let mut measure = NativeMeasure::new(&scene, base(), context(), [10., 10.]).unwrap();
    let prepared = measure.prepare(&scene, [10., 10.], context()).unwrap();
    assert_eq!(prepared.tree().nodes()[0].size, [44., 24.]);
    assert!(prepared.text().items().is_empty());
    assert_eq!(
        surface_box::inner([1., 1.], surface_box::inset(scene.root(), 10., context())),
        [0.; 2]
    );
}
