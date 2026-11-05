use super::*;
fn item(basis: f32) -> Item {
    Item {
        basis,
        minimum: 0.,
        maximum: 32768.,
        grow: 0.,
        shrink: 1.,
    }
}
fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 0.001, "{a} != {b}");
}
#[test]
fn growth_redistributes_after_maximum_and_fractional_growth_keeps_free_space() {
    let flow = arrange(
        &[
            Item {
                grow: 0.25,
                maximum: 30.,
                ..item(20.)
            },
            Item {
                grow: 0.25,
                ..item(20.)
            },
        ],
        100.,
        0.,
        Justify::Start,
    )
    .unwrap();
    close(flow.items[0].size, 30.);
    close(flow.items[1].size, 35.);
    let a = Item {
        grow: 1.,
        maximum: 40.,
        ..item(20.)
    };
    let b = Item {
        grow: 1.,
        ..item(20.)
    };
    let flow = arrange(&[a, b], 100., 0., Justify::Start).unwrap();
    close(flow.items[0].size, 40.);
    close(flow.items[1].size, 60.);
    let flow = arrange(
        &[Item {
            grow: 0.25,
            ..item(20.)
        }],
        100.,
        0.,
        Justify::End,
    )
    .unwrap();
    close(flow.items[0].size, 40.);
    close(flow.items[0].offset, 60.);
}
#[test]
fn shrinking_respects_basis_minimum_and_reports_overflow() {
    let flow = arrange(&[item(100.), item(50.)], 90., 0., Justify::Start).unwrap();
    close(flow.items[0].size, 60.);
    close(flow.items[1].size, 30.);
    let flow = arrange(
        &[
            Item {
                minimum: 70.,
                ..item(100.)
            },
            item(50.),
        ],
        90.,
        0.,
        Justify::Start,
    )
    .unwrap();
    close(flow.items[0].size, 70.);
    close(flow.items[1].size, 20.);
    let flow = arrange(
        &[
            Item {
                minimum: 70.,
                ..item(100.)
            },
            Item {
                minimum: 50.,
                ..item(50.)
            },
        ],
        90.,
        10.,
        Justify::Start,
    )
    .unwrap();
    close(flow.overflow, 40.);
    close(flow.items[1].offset, 80.);
}
#[test]
fn justification_preserves_gap_and_handles_empty_or_single_item() {
    for (justify, start, second) in [
        (Justify::Start, 0., 30.),
        (Justify::Center, 25., 55.),
        (Justify::End, 50., 80.),
        (Justify::Between, 0., 80.),
        (Justify::Around, 12.5, 67.5),
        (Justify::Evenly, 50. / 3., 30. + 100. / 3.),
    ] {
        let flow = arrange(&[item(20.), item(20.)], 100., 10., justify).unwrap();
        close(flow.items[0].offset, start);
        close(flow.items[1].offset, second);
    }
    assert!(arrange(&[], 100., 0., Justify::Between)
        .unwrap()
        .items
        .is_empty());
    close(
        arrange(&[item(20.)], 100., 0., Justify::Between)
            .unwrap()
            .items[0]
            .offset,
        0.,
    );
}
#[test]
fn invalid_or_unbounded_inputs_fail_before_layout() {
    assert_eq!(
        arrange(&[item(f32::NAN)], 100., 0., Justify::Start).unwrap_err(),
        Error::Input
    );
    assert_eq!(
        arrange(
            &[Item {
                minimum: 20.,
                maximum: 10.,
                ..item(10.)
            }],
            100.,
            0.,
            Justify::Start
        )
        .unwrap_err(),
        Error::Input
    );
    assert_eq!(
        arrange(&vec![item(1.); 2049], 100., 0., Justify::Start).unwrap_err(),
        Error::Limit
    );
}
