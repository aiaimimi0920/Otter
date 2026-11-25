use super::*;
#[test]
fn fixed_edges_are_emitted_once_around_forward_and_reverse_content() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for origins in [[0, 9, 18], [18, 0, 9]] {
            let frames: Vec<_> = origins
                .into_iter()
                .map(|origin| Positioned {
                    frame: image(axis, origin),
                    origin: i64::from(origin),
                })
                .collect();
            let result = stitch_fixed_edges(axis, &frames, 7, 5, &AtomicBool::new(false)).unwrap();
            assert_eq!(result.origin, -7);
            let expected_size = match axis {
                Axis::Vertical => (20, 82),
                Axis::Horizontal => (82, 20),
            };
            assert_eq!((result.width, result.height), expected_size);
            let first = image(axis, 0);
            let last = image(axis, 18);
            for y in 0..result.height {
                for x in 0..result.width {
                    let (along, cross) = match axis {
                        Axis::Vertical => (y, x),
                        Axis::Horizontal => (x, y),
                    };
                    let (source, line) = if along < 59 {
                        (&first, along)
                    } else if along < 77 {
                        (&last, along - 18)
                    } else {
                        (&first, along - 18)
                    };
                    let source_index = match axis {
                        Axis::Vertical => line * source.width + cross,
                        Axis::Horizontal => cross * source.width + line,
                    } as usize
                        * 4;
                    let target = (y * result.width + x) as usize * 4;
                    assert_eq!(
                        &result.pixels[target..target + 4],
                        &source.pixels[source_index..source_index + 4]
                    );
                }
            }
        }
    }
}
#[test]
fn changed_edges_invalid_bounds_and_output_origin_overflow_are_rejected() {
    let axis = Axis::Vertical;
    for row in [0, 63] {
        let original = image(axis, 9);
        let mut bytes = original.pixels.to_vec();
        bytes[row * 20 * 4] ^= 1;
        let frames = [
            Positioned {
                frame: image(axis, 0),
                origin: 0,
            },
            Positioned {
                frame: Frame::new(20, 64, bytes.into()).unwrap(),
                origin: 9,
            },
        ];
        assert!(matches!(
            stitch_fixed_edges(axis, &frames, 7, 5, &AtomicBool::new(false)),
            Err(Error::ChangedEdges)
        ));
    }
    let frames = [Positioned {
        frame: image(axis, 0),
        origin: i64::MIN,
    }];
    let cancel = AtomicBool::new(false);
    assert!(matches!(
        stitch_fixed_edges(axis, &frames, 7, 5, &cancel),
        Err(Error::OutputLimit)
    ));
    for edges in [(64, 0), (u32::MAX, 1)] {
        assert!(matches!(
            stitch_fixed_edges(axis, &frames, edges.0, edges.1, &cancel),
            Err(Error::Dimensions)
        ));
    }
}
fn image(axis: Axis, origin: u32) -> Frame {
    let (width, height) = match axis {
        Axis::Vertical => (20, 64),
        Axis::Horizontal => (64, 20),
    };
    let mut bytes = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let (line, cross) = match axis {
                Axis::Vertical => (y, x),
                Axis::Horizontal => (x, y),
            };
            let value = if line < 7 {
                11
            } else if line >= 59 {
                222
            } else {
                let mut seed: u32 = (line - 7 + origin) * 997 + cross * 7919 + 1;
                seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
                seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
                ((seed ^ (seed >> 16)) % 220) as u8
            };
            bytes.extend_from_slice(&[value, value / 2, 240 - value, 255]);
        }
    }
    Frame::new(width, height, bytes.into()).unwrap()
}
fn options() -> MatchOptions {
    MatchOptions {
        minimum_overlap: 24,
        maximum_shift: 20,
        maximum_mean_error: 3.0,
    }
}
#[test]
fn fixed_edges_are_excluded_in_both_axes_and_directions_without_copying() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let a = image(axis, 0);
        let b = image(axis, 9);
        for (first, second, delta) in [(&a, &b, 9), (&b, &a, -9)] {
            assert_eq!(
                match_fixed_edges(first, second, axis, options(), &AtomicBool::new(false)),
                Ok(FixedMatch::Motion {
                    leading: 7,
                    trailing: 5,
                    delta,
                    mean_error: 0.0
                })
            );
        }
        assert_eq!(a.dimensions(), image(axis, 0).dimensions());
        assert_eq!(&*a.pixels, &*image(axis, 0).pixels);
    }
}
#[test]
fn unchanged_cancelled_and_too_short_content_do_not_authorize_trimming() {
    let axis = Axis::Vertical;
    let a = image(axis, 0);
    let b = image(axis, 9);
    let cancel = AtomicBool::new(false);
    assert_eq!(
        match_fixed_edges(&a, &a, axis, options(), &cancel),
        Ok(FixedMatch::Unchanged)
    );
    assert_eq!(
        match_fixed_edges(&a, &b, axis, options(), &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
    let mut config = options();
    config.minimum_overlap = 60;
    assert_eq!(
        match_fixed_edges(&a, &b, axis, config, &cancel),
        Ok(FixedMatch::Unresolved(ApproximateMatch::NoOverlap))
    );
    config.minimum_overlap = 0;
    assert_eq!(
        match_fixed_edges(&a, &b, axis, config, &cancel),
        Err(Error::Dimensions)
    );
}
#[test]
fn internal_regions_validate_geometry_before_pixel_access() {
    let a = image(Axis::Vertical, 0);
    for range in [(5, 5), (6, 5), (0, 65), (u32::MAX, u32::MAX)] {
        assert_eq!(
            approximate::match_region(
                &a,
                &a,
                Axis::Vertical,
                options(),
                &AtomicBool::new(false),
                range
            ),
            Err(Error::Dimensions)
        );
    }
}
