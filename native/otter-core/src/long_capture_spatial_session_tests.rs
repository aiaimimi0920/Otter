use super::*;
#[test]
fn conversion_keeps_last_reference_and_negative_origin_on_both_axes() {
    let cancel = AtomicBool::new(false);
    let options = MatchOptions {
        minimum_overlap: 8,
        maximum_shift: 4,
        maximum_mean_error: 3.0,
    };
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for sign in [-1, 1] {
            let (dx, dy) = match axis {
                Axis::Vertical => (0, sign * 4),
                Axis::Horizontal => (sign * 4, 0),
            };
            let mut line = Session::new(axis, frame(100, 100), options).unwrap();
            line.append(frame(100 + dx, 100 + dy), &cancel).unwrap();
            let original = line.compose(&cancel).unwrap();
            let mut spatial = line.to_spatial(&cancel).unwrap();
            assert_eq!((spatial.frame_count(), spatial.position()), (2, (dx, dy)));
            assert_eq!(
                &*spatial.compose(&cancel).unwrap().pixels,
                &*original.pixels
            );
            assert_eq!(
                spatial.append(frame(104 + dx, 104 + dy), (4, 4), &cancel),
                Ok(SpatialAppend::Added {
                    x: dx + 4,
                    y: dy + 4
                })
            );
            assert_eq!(spatial.frame_count(), 3);
            assert_eq!(line.frame_count(), 2);
            assert_eq!(&*line.compose(&cancel).unwrap().pixels, &*original.pixels);
        }
    }
}
fn frame(x: i64, y: i64) -> Frame {
    let mut bytes = Vec::new();
    for row in 0..16 {
        for col in 0..16 {
            let value = ((x + col) * 17 + (y + row) * 37).rem_euclid(251) as u8;
            bytes.extend_from_slice(&[value, value / 2, 255 - value, 255]);
        }
    }
    Frame::new(16, 16, bytes.into()).unwrap()
}
fn session() -> SpatialSession {
    SpatialSession::new(
        Axis::Vertical,
        frame(100, 100),
        MatchOptions {
            minimum_overlap: 4,
            maximum_shift: 4,
            maximum_mean_error: 3.0,
        },
    )
    .unwrap()
}
#[test]
fn accepted_positions_accumulate_and_preview_preserves_holes_before_export_crop() {
    let cancel = AtomicBool::new(false);
    let mut session = session();
    assert_eq!(
        session.append(frame(104, 104), (4, 4), &cancel),
        Ok(SpatialAppend::Added { x: 4, y: 4 })
    );
    assert_eq!(session.position(), (4, 4));
    let preview = session.preview(&cancel).unwrap();
    assert_eq!(
        (preview.x, preview.y, preview.width, preview.height),
        (0, 0, 20, 20)
    );
    for y in 0..20 {
        for x in 0..20 {
            let offset = (y * 20 + x) * 4;
            let covered = (x < 16 && y < 16) || (x >= 4 && y >= 4);
            assert_eq!(preview.pixels[offset + 3], if covered { 255 } else { 0 });
            if covered {
                let value = (((100 + x) * 17 + (100 + y) * 37) % 251) as u8;
                assert_eq!(preview.pixels[offset], value);
            }
        }
    }
    let result = session.compose(&cancel).unwrap();
    assert_eq!(
        (result.x, result.y, result.width, result.height),
        (0, 0, 16, 16)
    );
    assert_eq!(&*result.pixels, frame(100, 100).bytes());
    assert_eq!(
        session.append(frame(100, 108), (-4, 4), &cancel),
        Ok(SpatialAppend::Added { x: 0, y: 8 })
    );
    assert_eq!(session.frame_count(), 3);
    assert_eq!(session.position(), (0, 8));
    assert_eq!(
        session.append(frame(100, 108), (0, 0), &cancel),
        Ok(SpatialAppend::Duplicate)
    );
    assert_eq!(session.frame_count(), 3);
}
#[test]
fn rejected_budget_and_cancelled_candidates_do_not_change_reference_or_output() {
    let cancel = AtomicBool::new(false);
    let mut session = session();
    let original = session.compose(&cancel).unwrap();
    assert_eq!(
        session.append(frame(104, 104), (i64::MAX, 0), &cancel),
        Ok(SpatialAppend::Rejected(ApproximateMatch::NoOverlap))
    );
    let budget = crate::byte_budget::Budget::new(1600);
    let occupied = budget.reserve(1).unwrap();
    assert_eq!(
        session.append_with_reserve(frame(104, 104), (4, 4), &cancel, |n| {
            assert_eq!(n, 1600);
            budget.reserve(n)
        }),
        Err(Error::Budget)
    );
    drop(occupied);
    assert_eq!(
        session.append_with_reserve(frame(104, 104), (4, 4), &cancel, |n| {
            cancel.store(true, Ordering::Release);
            budget.reserve(n)
        }),
        Err(Error::Cancelled)
    );
    assert_eq!(budget.used(), 0);
    cancel.store(false, Ordering::Release);
    assert_eq!((session.frame_count(), session.position()), (1, (0, 0)));
    assert_eq!(
        &*session.compose(&cancel).unwrap().pixels,
        &*original.pixels
    );
    assert_eq!(
        &*session.preview(&cancel).unwrap().pixels,
        &*original.pixels
    );
    assert_eq!(
        session.append_with_reserve(frame(104, 104), (4, 4), &cancel, |n| budget.reserve(n)),
        Ok(SpatialAppend::Added { x: 4, y: 4 })
    );
    assert_eq!(budget.used(), 0);
}
#[test]
fn preview_is_bounded_and_cancelled_requests_do_not_allocate_output() {
    let frame = Frame::new(512, 8, vec![123; 512 * 8 * 4].into()).unwrap();
    let session = SpatialSession::new(
        Axis::Horizontal,
        frame,
        MatchOptions {
            minimum_overlap: 4,
            maximum_shift: 4,
            maximum_mean_error: 3.0,
        },
    )
    .unwrap();
    let preview = session.preview(&AtomicBool::new(false)).unwrap();
    assert_eq!((preview.width, preview.height), (384, 6));
    assert!(preview.pixels.len() <= 384 * 384 * 4);
    assert!(preview
        .pixels
        .chunks_exact(4)
        .all(|p| p == [123, 123, 123, 255]));
    assert!(matches!(
        session.preview(&AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
}
