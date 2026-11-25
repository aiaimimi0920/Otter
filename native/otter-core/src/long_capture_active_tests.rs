//! First spatial/subsequent appends must publish identical axes/directions; failure advances no evidence.
use super::*;

fn frame(x: i64, y: i64) -> Frame {
    let pixels = (0..40)
        .flat_map(|row| {
            (0..32).flat_map(move |column| {
                let mut seed = (x + column) as u32 * 997 + (y + row) as u32 * 7919;
                seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
                seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
                let value = ((seed ^ (seed >> 16)) % 220) as u8;
                [value, value / 2, 240 - value, 255]
            })
        })
        .collect::<Vec<_>>();
    Frame::new(32, 40, pixels.into()).unwrap()
}

#[test]
fn first_spatial_append_commits_residual_direction_for_detecting_and_linear_sessions() {
    let cancel = AtomicBool::new(false);
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for residual in [-3, 3] {
            for locked in [false, true] {
                let (mut active, origin) = if locked {
                    let mut line = Session::new(
                        axis,
                        frame(100, 100),
                        MatchOptions {
                            minimum_overlap: 8,
                            maximum_shift: 8,
                            maximum_mean_error: 3.0,
                        },
                    )
                    .unwrap();
                    let origin = match axis {
                        Axis::Vertical => (0, 8),
                        Axis::Horizontal => (8, 0),
                    };
                    line.append(frame(100 + origin.0, 100 + origin.1), &cancel)
                        .unwrap();
                    (Active::Linear(line), origin)
                } else {
                    (
                        Active::Detecting(detect::Detect::new(frame(100, 100))),
                        (0, 0),
                    )
                };
                let movement = (3, 4);
                let next = match axis {
                    Axis::Vertical => (origin.0 + 3, origin.1 + 4 + residual),
                    Axis::Horizontal => (origin.0 + 3 + residual, origin.1 + 4),
                };
                assert!(matches!(
                    active.append_at(frame(100 + next.0, 100 + next.1), movement, &cancel),
                    Completion::SpatialAppend(Ok(SpatialAppend::Added { x, y })) if (x, y) == next
                ));
                let snapshot = active.snapshot();
                assert_eq!(snapshot.axis, Some(axis));
                assert_eq!(snapshot.direction, Direction::from_offset(axis, residual));
                assert_eq!(snapshot.spatial_position, Some(next));
                assert_eq!(snapshot.frame_count, if locked { 3 } else { 2 });

                // Duplication/rejection/cancellation never changes just-committed directions/coordinates.
                assert!(matches!(
                    active.append(frame(100 + next.0, 100 + next.1), &cancel),
                    Completion::SpatialAppend(Ok(SpatialAppend::Duplicate))
                ));
                assert_eq!(active.snapshot(), snapshot);
                assert!(matches!(
                    active.append_at(frame(100, 100), (i64::MAX, 0), &cancel),
                    Completion::SpatialAppend(Ok(SpatialAppend::Rejected(_)))
                ));
                assert_eq!(active.snapshot(), snapshot);
                assert!(matches!(
                    active.append_at(frame(100, 100), (1, 1), &AtomicBool::new(true)),
                    Completion::SpatialAppend(Err(Error::Cancelled))
                ));
                assert_eq!(active.snapshot(), snapshot);

                // Subsequent viewport-only movement preserves existing page direction.
                active.append_at(frame(102 + next.0, 102 + next.1), (2, 2), &cancel);
                assert_eq!(active.snapshot().direction, snapshot.direction);
                assert_eq!(
                    active.snapshot().spatial_position,
                    Some((next.0 + 2, next.1 + 2))
                );
            }
        }
    }
}

#[test]
fn physical_movement_does_not_invent_an_axis_or_direction() {
    let cancel = AtomicBool::new(false);
    let mut active = Active::Detecting(detect::Detect::new(frame(100, 100)));
    for step in 1..=2 {
        assert!(matches!(
            active.append_at(frame(100 + 3 * step, 100 + 4 * step), (3, 4), &cancel),
            Completion::SpatialAppend(Ok(SpatialAppend::Added { .. }))
        ));
        let snapshot = active.snapshot();
        assert_eq!(snapshot.axis, None);
        assert_eq!(snapshot.direction, None);
        assert_eq!(snapshot.spatial_position, Some((3 * step, 4 * step)));
    }
}
