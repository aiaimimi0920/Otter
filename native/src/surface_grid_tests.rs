use super::*;
use crate::surface_scene::Scene;
use serde_json::{json, Value};
#[path = "surface_grid_validation_tests.rs"]
mod validation;
fn context() -> Context {
    Context {
        axis: 400.,
        viewport: [800., 600.],
        container: [400., 300.],
        font: 16.,
        root_font: 16.,
        fraction: None,
    }
}
fn scene(layout: Value, children: Vec<Value>) -> Scene {
    Scene::parse(
        &serde_json::to_vec(
            &json!({"id":"root","type":"grid","layout":layout,"children":children}),
        )
        .unwrap(),
    )
    .unwrap()
}
fn child(id: &str, layout: Value) -> Value {
    json!({"id":id,"type":"view","layout":layout})
}
fn measured<'a>(scene: &'a Scene, sizes: &[[f32; 2]]) -> Vec<Measured<'a>> {
    scene
        .root()
        .visible_children()
        .zip(sizes)
        .map(|(n, size)| Measured {
            id: n.id(),
            size: *size,
            baseline: None,
        })
        .collect()
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.01, "{a} != {b}");
}
#[test]
fn grid_fixed_auto_and_fraction_tracks_have_distinct_sizes() {
    let s = scene(
        json!({"gridTemplateColumns":"20px minmax(0,1fr) auto","gridTemplateRows":"30px","gap":10}),
        vec![
            child("a", json!({})),
            child("b", json!({})),
            child("c", json!({})),
        ],
    );
    let m = measured(&s, &[[10., 10.], [30., 20.], [40., 10.]]);
    let v = layout(s.root(), &m, [200., 100.], context(), &|_| None, 200., true).unwrap();
    assert_eq!(v.cells, [[20., 30.], [120., 30.], [40., 30.]]);
    assert_eq!(
        v.boxes.iter().map(|p| p.origin).collect::<Vec<_>>(),
        [[0., 0.], [30., 0.], [160., 0.]]
    );
}
#[test]
fn grid_fraction_weights_freeze_lower_bounds_and_preserve_overflow() {
    let s = scene(
        json!({"gridTemplateColumns":"minmax(160px,.75fr) minmax(220px,1.25fr)"}),
        vec![child("a", json!({})), child("b", json!({}))],
    );
    let m = measured(&s, &[[0., 10.]; 2]);
    let small = layout(s.root(), &m, [300., 20.], context(), &|_| None, 300., true).unwrap();
    assert_eq!(small.cells, [[160., 20.], [220., 20.]]);
    let large = layout(
        s.root(),
        &m,
        [1000., 20.],
        context(),
        &|_| None,
        1000.,
        true,
    )
    .unwrap();
    assert_eq!(large.cells, [[375., 20.], [625., 20.]]);
}
#[test]
fn grid_repeat_two_axis_gap_and_implicit_equal_fraction_rows() {
    let s = scene(
        json!({"gridTemplateColumns":"repeat(2,minmax(0,1fr))","gridAutoRows":"minmax(0,1fr)","gap":"5px 10px"}),
        (0..4).map(|i| child(&format!("n{i}"), json!({}))).collect(),
    );
    let m = measured(&s, &[[10., 10.], [10., 20.], [10., 30.], [10., 40.]]);
    let fixed = layout(s.root(), &m, [110., 100.], context(), &|_| None, 110., true).unwrap();
    assert_eq!(fixed.cells, [[50., 47.5]; 4]);
    assert_eq!(fixed.boxes[3].origin, [60., 52.5]);
    let auto = layout(
        s.root(),
        &m,
        [110., 100.],
        context(),
        &|_| None,
        110.,
        false,
    )
    .unwrap();
    assert_eq!(auto.cells, [[50., 40.]; 4]);
    assert_eq!(auto.extent[1], 85.);
}
#[test]
fn grid_explicit_rows_and_negative_end_lines_reserve_before_auto_placement() {
    let s = scene(
        json!({"gridTemplateColumns":"repeat(3,minmax(0,1fr))","gridTemplateRows":"10px 20px 30px"}),
        vec![
            child("input", json!({})),
            child("intervals", json!({"gridRow":"2","gridColumn":"1/-1"})),
            child("refresh", json!({})),
            child("periods", json!({"gridRow":"3","gridColumn":"1/-1"})),
        ],
    );
    let m = measured(&s, &[[0., 0.]; 4]);
    let v = layout(s.root(), &m, [300., 60.], context(), &|_| None, 300., true).unwrap();
    assert_eq!(
        v.boxes.iter().map(|p| p.origin).collect::<Vec<_>>(),
        [[0., 0.], [0., 10.], [100., 0.], [0., 30.]]
    );
    assert_eq!(v.cells[1], [300., 20.]);
}
#[test]
fn grid_span_and_line_range_keep_cell_percentages_and_self_alignment() {
    let s = scene(
        json!({"gridTemplateColumns":"100px 100px","gridTemplateRows":"20px 30px","gap":10}),
        vec![
            child(
                "span",
                json!({"gridColumn":"2","gridRow":"1/3","width":"50%","height":10,"justifySelf":"end","alignSelf":"center"}),
            ),
            child("a", json!({})),
            child("b", json!({})),
        ],
    );
    let m = measured(&s, &[[10., 10.]; 3]);
    let v = layout(s.root(), &m, [210., 60.], context(), &|_| None, 210., true).unwrap();
    assert_eq!(v.cells[0], [100., 60.]);
    assert_eq!(v.boxes[0].size, [50., 10.]);
    assert_eq!(v.boxes[0].origin, [160., 25.]);
    assert_eq!(v.boxes[2].origin, [0., 30.]);
    assert_eq!(placement::parse(Some(&json!("span 2")), 3).unwrap().span, 2);
    assert_eq!(
        placement::parse(Some(&json!("span 2 / 4")), 3)
            .unwrap()
            .start,
        Some(1)
    );
}
#[test]
fn grid_content_and_item_alignment_are_separate_and_baseline_groups_expand_auto_row() {
    let s = scene(
        json!({"gridTemplateColumns":"40px 40px","align":"baseline","alignContent":"center","justify":"end"}),
        vec![child("a", json!({})), child("b", json!({}))],
    );
    let mut m = measured(&s, &[[20., 20.]; 2]);
    m[0].baseline = Some(8.);
    m[1].baseline = Some(15.);
    let v = layout(s.root(), &m, [100., 100.], context(), &|_| None, 100., true).unwrap();
    assert_eq!(v.cells[0], [40., 27.]);
    near(v.boxes[0].origin[1], 43.5);
    near(v.boxes[1].origin[1], 36.5);
    assert_eq!(v.boxes[0].origin[0], 20.);
}
#[test]
fn grid_hidden_children_leave_placement_and_explicit_overlap_keeps_dom_order() {
    let s = scene(
        json!({"gridTemplateColumns":"40px"}),
        vec![
            json!({"id":"hidden","type":"view","props":{"visible":false}}),
            child("a", json!({"gridColumn":"1","gridRow":"1"})),
            child("b", json!({"gridColumn":"1","gridRow":"1"})),
        ],
    );
    let m = measured(&s, &[[10., 10.]; 2]);
    let v = layout(s.root(), &m, [40., 20.], context(), &|_| None, 40., true).unwrap();
    assert_eq!(
        v.boxes.iter().map(|p| p.node.id()).collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(v.boxes[0].origin, v.boxes[1].origin);
}
#[test]
fn grid_rejects_unknown_syntax_limits_nonfinite_and_foreign_measurements() {
    for columns in [
        "repeat(65,1fr)",
        "repeat(auto-fill,1fr)",
        "minmax(1fr,2fr)",
        "calc(10px)",
        "subgrid",
        "[name] 1fr",
        "repeat(2,repeat(2,1fr))",
    ] {
        let s = scene(
            json!({"gridTemplateColumns":columns}),
            vec![child("a", json!({}))],
        );
        assert!(
            layout(
                s.root(),
                &measured(&s, &[[10., 10.]]),
                [100., 100.],
                context(),
                &|_| None,
                100.,
                true
            )
            .is_err(),
            "{columns}"
        );
    }
    for child_layout in [
        json!({"gridColumn":"65"}),
        json!({"gridRow":"0"}),
        json!({"gridRow":"-2"}),
        json!({"gridColumn":"span 65"}),
        json!({"position":"absolute"}),
        json!({"gridRow":"1/3","alignSelf":"baseline"}),
    ] {
        let s = scene(json!({}), vec![child("a", child_layout.clone())]);
        assert!(
            layout(
                s.root(),
                &measured(&s, &[[10., 10.]]),
                [100., 100.],
                context(),
                &|_| None,
                100.,
                true
            )
            .is_err(),
            "{child_layout}"
        );
    }
    let s = scene(json!({}), vec![child("a", json!({}))]);
    let mut m = measured(&s, &[[10., 10.]]);
    m[0].size[0] = f32::NAN;
    assert!(layout(s.root(), &m, [100., 100.], context(), &|_| None, 100., true).is_err());
    m[0].size[0] = 10.;
    m[0].id = "foreign";
    assert!(layout(s.root(), &m, [100., 100.], context(), &|_| None, 100., true).is_err());
}
