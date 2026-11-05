use super::*;
fn frame(x: i64, y: i64) -> Frame {
    let mut pixels = Vec::new();
    for row in 0..40 {
        for column in 0..32 {
            let mut seed = (x + column) as u32 * 997 + (y + row) as u32 * 7919;
            seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
            seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
            let v = ((seed ^ (seed >> 16)) % 220) as u8;
            pixels.extend_from_slice(&[v, v / 2, 240 - v, 255]);
        }
    }
    Frame::new(32, 40, pixels.into()).unwrap()
}
fn options() -> MatchOptions {
    MatchOptions {
        minimum_overlap: 8,
        maximum_shift: 8,
        maximum_mean_error: 3.0,
    }
}
#[test]
fn periodic_residual_motion_is_not_reported_as_a_unique_position() {
    let make = |phase: u32| {
        let pixels = (0..40)
            .flat_map(|y| {
                let value = if (y + phase).is_multiple_of(2) {
                    20
                } else {
                    200
                };
                (0..32).flat_map(move |_| [value, value, value, 255])
            })
            .collect::<Vec<_>>();
        Frame::new(32, 40, pixels.into()).unwrap()
    };
    assert_eq!(
        match_spatial(
            &make(0),
            &make(1),
            (0, 0),
            Axis::Vertical,
            options(),
            &AtomicBool::new(false)
        ),
        Ok(SpatialMatch::Rejected(ApproximateMatch::Ambiguous))
    );
}
#[test]
fn known_diagonal_translation_and_residual_scroll_use_original_pixel_strides() {
    let cancel = AtomicBool::new(false);
    let first = frame(100, 100);
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for movement in [(5, 7), (-5, 7), (5, -7), (-5, -7), (0, 0)] {
            for residual in [-3, 0, 3] {
                let (x, y) = match axis {
                    Axis::Vertical => (movement.0, movement.1 + residual),
                    Axis::Horizontal => (movement.0 + residual, movement.1),
                };
                let result = match_spatial(
                    &first,
                    &frame(100 + x, 100 + y),
                    movement,
                    axis,
                    options(),
                    &cancel,
                )
                .unwrap();
                assert_eq!(
                    result,
                    SpatialMatch::Offset {
                        x,
                        y,
                        mean_error: 0.0
                    }
                );
            }
        }
    }
}
#[test]
fn invalid_overlap_extreme_movement_and_cancellation_are_rejected() {
    let first = frame(100, 100);
    let cancel = AtomicBool::new(false);
    for movement in [
        (i64::MIN, 0),
        (i64::MAX, 0),
        (0, i64::MIN),
        (32, 0),
        (0, -40),
    ] {
        assert_eq!(
            match_spatial(&first, &first, movement, Axis::Vertical, options(), &cancel),
            Ok(SpatialMatch::Rejected(ApproximateMatch::NoOverlap))
        );
    }
    assert_eq!(
        match_spatial(
            &first,
            &first,
            (0, 0),
            Axis::Vertical,
            options(),
            &AtomicBool::new(true)
        ),
        Err(Error::Cancelled)
    );
    for maximum_mean_error in [f64::NAN, f64::INFINITY, -1.0, 13.0] {
        assert_eq!(
            match_spatial(
                &first,
                &first,
                (0, 0),
                Axis::Vertical,
                MatchOptions {
                    maximum_mean_error,
                    ..options()
                },
                &cancel
            ),
            Err(Error::Dimensions)
        );
    }
    assert!(View::new(&first, u32::MAX, 0, 2, 2).is_err());
    assert!(View::new(&first, 0, 0, 0, 2).is_err());
    assert!(View::new(&first, 0, 39, 2, 2).is_err());
}
#[test]
fn flat_known_translation_is_valid_but_different_flat_frames_cannot_infer_scroll() {
    let cancel = AtomicBool::new(false);
    let a = Frame::new(32, 40, vec![0; 32 * 40 * 4].into()).unwrap();
    let b = Frame::new(32, 40, vec![100; 32 * 40 * 4].into()).unwrap();
    assert_eq!(
        match_spatial(&a, &a, (4, -3), Axis::Vertical, options(), &cancel),
        Ok(SpatialMatch::Offset {
            x: 4,
            y: -3,
            mean_error: 0.0
        })
    );
    assert_eq!(
        match_spatial(&a, &b, (4, -3), Axis::Vertical, options(), &cancel),
        Ok(SpatialMatch::Rejected(ApproximateMatch::LowTexture))
    );
}
