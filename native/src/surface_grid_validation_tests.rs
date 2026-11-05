//! 稀疏放置、空网格和新属性严格准入的反例；通用长度默认值沿既有 Scene 合同。
use super::*;
#[test]
fn grid_axis_overlapping_spans_charge_work_even_with_a_small_final_rectangle() {
    let tracks = vec![tracks::Track::AUTO; 2048];
    let areas = vec![
        placement::Area {
            start: [0, 0],
            span: [1, 2048]
        };
        129
    ];
    let result = axis::resolve(
        &tracks,
        &areas,
        &[0.; 129],
        1,
        axis::Space {
            available: None,
            gap: 0.,
            context: context(),
            distribution: axis::distribution(None).unwrap(),
        },
    );
    assert!(matches!(result, Err(Error::Limit)));
}
#[test]
fn grid_baseline_synthesis_uses_the_same_constrained_height_as_final_placement() {
    for properties in [json!({"height":20}), json!({"minHeight":20})] {
        let s = scene(
            json!({"align":"baseline", "gridTemplateColumns":"40px 40px"}),
            vec![child("box", properties), child("text", json!({}))],
        );
        let mut m = measured(&s, &[[10., 10.]; 2]);
        m[1].baseline = Some(5.);
        let v = layout(s.root(), &m, [80., 100.], context(), &|_| None, 80., false).unwrap();
        assert_eq!(v.cells[0][1], 25.);
        assert_eq!(v.boxes[0].origin[1], 0.);
        assert_eq!(v.boxes[1].origin[1], 15.);
        assert_eq!(v.boxes[0].size[1], 20.);
    }
    let s = scene(
        json!({"align":"baseline"}),
        vec![child("box", json!({"height":"50%"}))],
    );
    assert!(layout(
        s.root(),
        &measured(&s, &[[10., 10.]]),
        [80., 100.],
        context(),
        &|_| None,
        80.,
        true
    )
    .is_err());
}
#[test]
fn grid_row_locked_sparse_cursor_does_not_backfill_earlier_holes() {
    let s = scene(
        json!({"gridTemplateColumns":"10px 10px 10px","gridAutoColumns":"10px"}),
        vec![
            child("fixed", json!({"gridRow":"1","gridColumn":"2"})),
            child("wide", json!({"gridRow":"1","gridColumn":"span 2"})),
            child("next", json!({"gridRow":"1"})),
        ],
    );
    let v = layout(
        s.root(),
        &measured(&s, &[[0., 0.]; 3]),
        [100., 20.],
        context(),
        &|_| None,
        100.,
        true,
    )
    .unwrap();
    assert_eq!(
        v.boxes.iter().map(|p| p.origin[0]).collect::<Vec<_>>(),
        [10., 20., 40.]
    );
    let s = scene(
        json!({}),
        vec![child("bad", json!({"gridRow":"2048 / span 2"}))],
    );
    assert!(layout(
        s.root(),
        &measured(&s, &[[0., 0.]]),
        [100., 20.],
        context(),
        &|_| None,
        100.,
        true
    )
    .is_err());
}
#[test]
fn grid_empty_has_no_implicit_track_and_retains_only_explicit_axes() {
    for (properties, extent) in [
        (json!({}), [0., 0.]),
        (
            json!({"gridTemplateColumns":"10px 20px", "gap":5}),
            [35., 0.],
        ),
        (json!({"padding":3}), [6., 6.]),
    ] {
        let s = scene(properties, vec![]);
        let v = layout(s.root(), &[], [100., 80.], context(), &|_| None, 100., true).unwrap();
        assert_eq!(v.extent, extent);
        assert!(v.boxes.is_empty() && v.cells.is_empty());
    }
}
#[test]
fn grid_alignment_rejects_wrong_json_types_instead_of_using_defaults() {
    for value in [json!(true), json!(7), json!({}), json!([]), Value::Null] {
        for key in ["align", "justifyItems", "alignContent", "justify"] {
            let mut properties = json!({});
            properties[key] = value.clone();
            let s = scene(properties, vec![child("a", json!({}))]);
            assert!(
                layout(
                    s.root(),
                    &measured(&s, &[[10., 10.]]),
                    [100., 80.],
                    context(),
                    &|_| None,
                    100.,
                    true
                )
                .is_err(),
                "{key}: {value}"
            );
        }
        for key in ["alignSelf", "justifySelf"] {
            let mut properties = json!({});
            properties[key] = value.clone();
            let s = scene(json!({}), vec![child("a", properties)]);
            assert!(
                layout(
                    s.root(),
                    &measured(&s, &[[10., 10.]]),
                    [100., 80.],
                    context(),
                    &|_| None,
                    100.,
                    true
                )
                .is_err(),
                "{key}: {value}"
            );
        }
    }
}
#[test]
fn grid_invalid_context_and_indefinite_percent_tracks_fail_closed() {
    let s = scene(
        json!({"gridTemplateRows":"50%"}),
        vec![child("a", json!({}))],
    );
    let m = measured(&s, &[[10., 10.]]);
    assert!(layout(s.root(), &m, [100., 80.], context(), &|_| None, 100., false).is_err());
    let v = layout(s.root(), &m, [100., 80.], context(), &|_| None, 100., true).unwrap();
    assert_eq!(v.cells[0][1], 40.);
    let mut invalid = context();
    invalid.viewport[1] = f32::NAN;
    assert!(layout(s.root(), &m, [100., 80.], invalid, &|_| None, 100., true).is_err());
    assert!(layout(
        s.root(),
        &m,
        [100., 80.],
        context(),
        &|_| Some(f32::NAN),
        100.,
        true
    )
    .is_err());
}
#[test]
fn grid_content_alignment_preserves_center_and_end_overflow() {
    for (justify, origin) in [("center", -25.), ("end", -50.)] {
        let s = scene(
            json!({"gridTemplateColumns":"100px", "justify":justify}),
            vec![child("a", json!({}))],
        );
        let v = layout(
            s.root(),
            &measured(&s, &[[10., 10.]]),
            [50., 20.],
            context(),
            &|_| None,
            50.,
            true,
        )
        .unwrap();
        assert_eq!(v.boxes[0].origin[0], origin);
        assert_eq!(v.cells[0][0], 100.);
    }
}
