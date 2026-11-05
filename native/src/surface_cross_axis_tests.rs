use super::*;
fn item(intrinsic: f32) -> Item {
    Item {
        intrinsic,
        explicit: None,
        minimum: 0.,
        maximum: 32768.,
        baseline: None,
    }
}
#[test]
fn stretching_respects_explicit_size_and_constraints() {
    let result = arrange(
        &[
            item(10.),
            Item {
                explicit: Some(20.),
                ..item(10.)
            },
            Item {
                maximum: 30.,
                ..item(10.)
            },
            Item {
                minimum: 70.,
                ..item(10.)
            },
        ],
        50.,
        Align::Stretch,
    )
    .unwrap();
    assert_eq!(
        result.items.iter().map(|p| p.size).collect::<Vec<_>>(),
        [50., 20., 30., 70.]
    );
    assert_eq!(result.overflow_after, 20.);
    assert_eq!(result.overflow_before, 0.);
}
#[test]
fn baseline_uses_measured_ascent_and_synthesizes_nontext_bottom_edge() {
    let result = arrange(
        &[
            Item {
                baseline: Some(12.),
                ..item(20.)
            },
            Item {
                baseline: Some(24.),
                ..item(30.)
            },
            item(10.),
        ],
        30.,
        Align::Baseline,
    )
    .unwrap();
    assert_eq!(result.baseline, Some(24.));
    assert_eq!(
        result.items.iter().map(|p| p.offset).collect::<Vec<_>>(),
        [12., 0., 14.]
    );
    assert_eq!(result.overflow_after, 2.);
    for (p, baseline) in result.items.iter().zip([12., 24., 10.]) {
        assert_eq!(p.offset + baseline, 24.);
    }
}
#[test]
fn center_and_end_keep_negative_offsets_instead_of_silent_clipping() {
    let centered = arrange(&[item(80.)], 40., Align::Center).unwrap();
    assert_eq!(centered.items[0].offset, -20.);
    assert_eq!(centered.overflow_before, 20.);
    assert_eq!(centered.overflow_after, 20.);
    let end = arrange(&[item(80.)], 40., Align::End).unwrap();
    assert_eq!(end.items[0].offset, -40.);
    assert_eq!(end.overflow_before, 40.);
    assert_eq!(end.overflow_after, 0.);
    assert!(arrange(&[], 40., Align::Baseline)
        .unwrap()
        .baseline
        .is_none());
}
#[test]
fn invalid_geometry_is_rejected_before_allocating_placements() {
    assert_eq!(
        arrange(
            &[Item {
                baseline: Some(f32::NAN),
                ..item(10.)
            }],
            40.,
            Align::Baseline
        )
        .unwrap_err(),
        Error::Input
    );
    assert_eq!(
        arrange(
            &[Item {
                maximum: 1.,
                minimum: 2.,
                ..item(10.)
            }],
            40.,
            Align::Start
        )
        .unwrap_err(),
        Error::Input
    );
    assert_eq!(
        arrange(&vec![item(1.); 2049], 40., Align::Start).unwrap_err(),
        Error::Limit
    );
}
