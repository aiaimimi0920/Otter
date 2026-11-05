use super::*;
fn frame(x0: u32, y0: u32, fixed: Option<Axis>) -> Frame {
    let mut pixels = Vec::new();
    for y in 0..64u32 {
        for x in 0..48u32 {
            let edge = match fixed {
                Some(Axis::Vertical) => !(3..60).contains(&y),
                Some(Axis::Horizontal) => !(3..44).contains(&x),
                None => false,
            };
            let value = if edge {
                13
            } else {
                let mut seed = (x0 + x).wrapping_mul(7919) ^ (y0 + y).wrapping_mul(104729);
                seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
                seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
                ((seed ^ (seed >> 16)) % 220) as u8
            };
            pixels.extend_from_slice(&[value, value / 2, 240 - value, 255]);
        }
    }
    Frame::new(48, 64, pixels.into()).unwrap()
}
#[test]
fn all_axes_directions_reverse_without_replacing_the_accepted_baseline() {
    let cancel = AtomicBool::new(false);
    for (axis, sign, expected, reverse) in [
        (Axis::Vertical, 1, Direction::Down, Direction::Up),
        (Axis::Vertical, -1, Direction::Up, Direction::Down),
        (Axis::Horizontal, 1, Direction::Right, Direction::Left),
        (Axis::Horizontal, -1, Direction::Left, Direction::Right),
    ] {
        for fixed in [false, true] {
            let make = |offset: i32| {
                let position = (32 + offset * sign) as u32;
                frame(
                    if axis == Axis::Horizontal {
                        position
                    } else {
                        32
                    },
                    if axis == Axis::Vertical { position } else { 32 },
                    fixed.then_some(axis),
                )
            };
            let mut detect = Detect::new(make(0));
            assert_eq!(detect.snapshot().axis, None);
            assert_eq!(detect.snapshot().direction, None);
            assert_eq!(detect.append(make(0), &cancel).unwrap(), Append::Duplicate);
            assert!(matches!(
                detect.append(make(8), &cancel),
                Ok(Append::Added { .. })
            ));
            assert_eq!(detect.snapshot().axis, Some(axis));
            assert_eq!(detect.snapshot().direction, Some(expected));
            if fixed {
                assert!(matches!(
                    detect.compose(&cancel),
                    Err(Error::UnconfirmedEdges)
                ));
            }
            detect.append(make(16), &cancel).unwrap();
            let expected_pixels = detect.compose(&cancel).unwrap();
            assert_eq!(detect.append(make(16), &cancel).unwrap(), Append::Duplicate);
            assert_eq!(detect.snapshot().direction, Some(expected));
            detect.append(make(8), &cancel).unwrap();
            assert_eq!(detect.snapshot().direction, Some(reverse));
            let output = detect.compose(&cancel).unwrap();
            assert_eq!(&*output.pixels, &*expected_pixels.pixels);
            assert_eq!(
                (output.width, output.height),
                if axis == Axis::Vertical {
                    (48, 80)
                } else {
                    (64, 64)
                }
            );
        }
    }
}
#[test]
fn ambiguous_diagonal_and_changed_flat_frames_leave_axis_unknown() {
    let make = |offset: u32| {
        Frame::new(
            48,
            64,
            (0..64u32)
                .flat_map(|y| {
                    (0..48u32).flat_map(move |x| {
                        let value = ((x + y + offset) * 37 % 220) as u8;
                        [value, value / 2, 240 - value, 255]
                    })
                })
                .collect::<Vec<_>>()
                .into(),
        )
        .unwrap()
    };
    let cancel = AtomicBool::new(false);
    let initial = make(0);
    let mut detect = Detect::new(initial.clone());
    assert!(matches!(
        detect.append(make(8), &cancel),
        Ok(Append::Rejected(_))
    ));
    let flat = Frame::new(48, 64, vec![0; 48 * 64 * 4].into()).unwrap();
    assert!(matches!(
        detect.append(flat, &cancel),
        Ok(Append::Rejected(_))
    ));
    assert_eq!(detect.snapshot().axis, None);
    assert_eq!(detect.snapshot().frame_count, 1);
    assert_eq!(&*detect.compose(&cancel).unwrap().pixels, initial.bytes());
}
#[test]
fn cross_axis_and_cancelled_frames_preserve_axis_direction_and_pixels() {
    let cancel = AtomicBool::new(false);
    let mut detect = Detect::new(frame(32, 32, None));
    detect.append(frame(32, 40, None), &cancel).unwrap();
    let before = detect.snapshot();
    let output = detect.compose(&cancel).unwrap();
    assert_eq!(
        detect.append(frame(40, 40, None), &cancel),
        Err(Error::CrossAxis)
    );
    assert_eq!(detect.snapshot(), before);
    assert_eq!(
        detect.append(frame(32, 48, None), &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
    assert_eq!(detect.snapshot(), before);
    assert_eq!(&*detect.compose(&cancel).unwrap().pixels, &*output.pixels);
    detect.append(frame(32, 48, None), &cancel).unwrap();
    assert_eq!(detect.snapshot().frame_count, 3);
}
#[test]
fn initial_preview_and_final_share_pixels_without_fabricating_axis_or_direction() {
    let first = frame(0, 0, None);
    let detect = Detect::new(first.clone());
    let cancel = AtomicBool::new(false);
    assert!(detect
        .compose(&cancel)
        .unwrap()
        .pixels
        .same_storage(&first.pixels));
    assert_eq!(detect.snapshot().axis, None);
    assert_eq!(detect.snapshot().direction, None);
    assert!(matches!(detect.to_spatial(&cancel), Err(Error::CrossAxis)));
    assert!(detect.preview(&cancel).is_ok());
    assert!(matches!(
        detect.compose(&AtomicBool::new(true)),
        Err(Error::Cancelled)
    ));
}
