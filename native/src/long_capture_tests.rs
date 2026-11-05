use super::*;

#[test]
fn nonuniform_rows_and_columns_use_correct_source_offsets_and_keep_bgra() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let make = |origin: i64| {
            let mut bytes = Vec::new();
            for y in 0..4 {
                for x in 0..4 {
                    let (along, cross) = match axis {
                        Axis::Vertical => (y, x),
                        Axis::Horizontal => (x, y),
                    };
                    bytes.extend_from_slice(&[(origin + along) as u8, cross as u8, 17, 255]);
                }
            }
            Positioned {
                frame: Frame::new(4, 4, bytes.into()).unwrap(),
                origin,
            }
        };
        let frames = [make(10), make(12), make(8)];
        let result = stitch(axis, &frames, &AtomicBool::new(false)).unwrap();
        for y in 0..result.height {
            for x in 0..result.width {
                let offset = ((y * result.width + x) * 4) as usize;
                let (along, cross) = match axis {
                    Axis::Vertical => (y, x),
                    Axis::Horizontal => (x, y),
                };
                assert_eq!(
                    &result.pixels[offset..offset + 4],
                    &[(result.origin + along as i64) as u8, cross as u8, 17, 255]
                );
            }
        }
    }
}
fn frame(width: u32, height: u32, color: u8, origin: i64) -> Positioned {
    Positioned {
        frame: Frame::new(
            width,
            height,
            vec![color; (width * height * 4) as usize].into(),
        )
        .unwrap(),
        origin,
    }
}
#[test]
fn vertical_reverse_and_revisited_regions_preserve_first_coverage() {
    let frames = [
        frame(2, 3, 1, 0),
        frame(2, 3, 2, 2),
        frame(2, 3, 3, -2),
        frame(2, 3, 4, 1),
    ];
    let result = stitch(Axis::Vertical, &frames, &AtomicBool::new(false)).unwrap();
    assert_eq!((result.width, result.height, result.origin), (2, 7, -2));
    let colors: Vec<_> = result.pixels.chunks_exact(8).map(|row| row[0]).collect();
    assert_eq!(colors, [3, 3, 1, 1, 1, 2, 2]);
    assert_eq!(frames[0].frame.pixels[0], 1);
}
#[test]
fn horizontal_spanning_frame_extends_both_sides_without_replacing_middle() {
    let frames = [frame(2, 2, 1, 10), frame(6, 2, 2, 8), frame(2, 2, 3, 10)];
    let result = stitch(Axis::Horizontal, &frames, &AtomicBool::new(false)).unwrap();
    assert_eq!((result.width, result.height, result.origin), (6, 2, 8));
    for row in result.pixels.chunks_exact(24) {
        assert_eq!(
            row.chunks_exact(4)
                .map(|pixel| pixel[0])
                .collect::<Vec<_>>(),
            [2, 2, 1, 1, 2, 2]
        );
    }
}
#[test]
fn invalid_shape_gap_overflow_and_counts_fail_before_output_allocation() {
    assert_eq!(frame_pixels(0, 1), Err(Error::Dimensions));
    assert_eq!(frame_pixels(32769, 1), Err(Error::Dimensions));
    assert_eq!(frame_pixels(32768, 32768), Err(Error::Dimensions));
    assert_eq!(output_bytes(131073, 1), Err(Error::OutputLimit));
    assert!(matches!(
        Frame::new(2, 2, vec![0; 3].into()),
        Err(Error::Pixels)
    ));
    let cancel = AtomicBool::new(false);
    for (frames, expected) in [
        (vec![], Error::Empty),
        (vec![frame(1, 1, 1, 0), frame(2, 1, 1, 0)], Error::CrossAxis),
        (vec![frame(1, 1, 1, 0), frame(1, 1, 1, 2)], Error::Gap),
        (vec![frame(1, 1, 1, i64::MAX)], Error::OutputLimit),
    ] {
        assert!(
            matches!(compose::stitch(Axis::Vertical, &frames, &cancel, |_| panic!("invalid plan allocated")), Err(error) if error == expected)
        );
    }
    let frames: Vec<_> = (0..513).map(|_| frame(1, 1, 1, 0)).collect();
    assert!(matches!(
        stitch(Axis::Vertical, &frames, &cancel),
        Err(Error::FrameLimit)
    ));
}
#[test]
fn cancellation_and_budget_failure_release_output_without_mutating_shared_frames() {
    let frames = [frame(2, 3, 7, 0), frame(2, 3, 8, 2)];
    let copy = frames[0].frame.clone();
    assert!(copy.pixels.same_storage(&frames[0].frame.pixels));
    assert!(matches!(
        stitch(Axis::Vertical, &frames, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        compose::stitch(Axis::Vertical, &frames, &AtomicBool::new(false), |_| None),
        Err(Error::Budget)
    ));
    let budget = crate::byte_budget::Budget::new(100);
    let cancel = AtomicBool::new(false);
    assert!(matches!(
        compose::stitch(Axis::Vertical, &frames, &cancel, |bytes| {
            cancel.store(true, std::sync::atomic::Ordering::Release);
            budget.reserve(bytes)
        }),
        Err(Error::Cancelled)
    ));
    assert_eq!(budget.used(), 0);
    assert_eq!(copy.pixels[0], 7);
}

#[test]
fn duplicate_frames_share_output_but_still_count_toward_input_admission() {
    let first = frame(2, 3, 7, -9);
    let frames = [
        Positioned {
            frame: first.frame.clone(),
            origin: -9,
        },
        first,
    ];
    let result = compose::stitch(Axis::Vertical, &frames, &AtomicBool::new(false), |_| {
        panic!("duplicate allocated")
    })
    .unwrap();
    assert!(result.pixels.same_storage(&frames[0].frame.pixels));
    assert_eq!(result.origin, -9);
    let image = Frame::new(1024, 1024, vec![0; 1024 * 1024 * 4].into()).unwrap();
    let repeated: Vec<_> = (0..129)
        .map(|_| Positioned {
            frame: image.clone(),
            origin: 0,
        })
        .collect();
    assert!(matches!(
        compose::stitch(
            Axis::Vertical,
            &repeated,
            &AtomicBool::new(false),
            |_| panic!("over-budget allocated")
        ),
        Err(Error::InputLimit)
    ));
    assert_eq!(output_bytes(1, 131072), Ok(524288));
    assert_eq!(output_bytes(131072, 1025), Err(Error::OutputLimit));
}
