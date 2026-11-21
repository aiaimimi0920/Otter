//! 留白误认固定栏的四向回归；验证最终像素和坐标，真实固定栏变化仍拒绝提交。
use super::*;

fn pixels(axis: Axis, origin: u32, blank: (u32, u32), reverse: bool, length: u32) -> Vec<u8> {
    let (width, height) = if axis == Axis::Vertical {
        (20, length)
    } else {
        (length, 20)
    };
    let mut bytes = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let (line, cross) = if axis == Axis::Vertical {
                (y, x)
            } else {
                (x, y)
            };
            let line = if reverse { length - 1 - line } else { line };
            let value = if line < 7 {
                11
            } else if line >= length - 5 {
                222
            } else {
                let world = line - 7 + origin;
                if (blank.0..=blank.1).contains(&world) {
                    128
                } else {
                    let mut seed = world * 997 + cross * 7919 + 1;
                    seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
                    seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
                    ((seed ^ (seed >> 16)) % 220) as u8
                }
            };
            bytes.extend_from_slice(&[value, value / 2, 240 - value, 255]);
        }
    }
    bytes
}
#[test]
fn scrolling_blank_margins_can_shrink_or_grow_without_changing_fixed_chrome() {
    let cancel = AtomicBool::new(false);
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            for (origins, blank) in [([0, 9, 18], (0, 12)), ([9, 18, 27], (18, 30))] {
                let make = |origin| {
                    let (width, height) = if axis == Axis::Vertical {
                        (20, 64)
                    } else {
                        (64, 20)
                    };
                    Frame::new(
                        width,
                        height,
                        pixels(axis, origin, blank, reverse, 64).into(),
                    )
                    .unwrap()
                };
                let options = MatchOptions {
                    minimum_overlap: 24,
                    maximum_shift: 20,
                    maximum_mean_error: 3.0,
                };
                let mut session = Session::new(axis, make(origins[0]), options).unwrap();
                for (step, origin) in origins.into_iter().enumerate().skip(1) {
                    let expected = step as i64 * if reverse { -9 } else { 9 };
                    assert_eq!(
                        session.append(make(origin), &cancel),
                        Ok(Append::Added { origin: expected })
                    );
                    if step == 1 {
                        let leading = if blank.0 == 0 { 11 } else { 7 };
                        assert_eq!(
                            session.edges,
                            Some(if reverse { (5, leading) } else { (leading, 5) })
                        );
                    }
                }
                assert_eq!(
                    session.confirmed_edges(),
                    Some(if reverse { (5, 7) } else { (7, 5) })
                );
                let output = session.compose(&cancel).unwrap();
                assert_eq!(output.origin, if reverse { -23 } else { -7 });
                assert_eq!(
                    &*output.pixels,
                    pixels(axis, origins[0], blank, reverse, 82)
                );
                assert_eq!(&*session.preview(&cancel).unwrap().pixels, &*output.pixels);
                let clean = make(origins[2]);
                let mut changed = clean.bytes().to_vec();
                changed[0] ^= 1;
                let changed = Frame::new(clean.width, clean.height, changed.into()).unwrap();
                assert_eq!(session.append(changed, &cancel), Err(Error::ChangedEdges));
                assert_eq!(session.frame_count(), 3);
                assert_eq!(&*session.compose(&cancel).unwrap().pixels, &*output.pixels);
            }
        }
    }
}

#[test]
fn disappearing_fixed_chrome_cannot_be_reinterpreted_as_historical_content() {
    let cancel = AtomicBool::new(false);
    for axis in [Axis::Vertical, Axis::Horizontal] {
        for reverse in [false, true] {
            let (width, height) = if axis == Axis::Vertical {
                (20, 64)
            } else {
                (64, 20)
            };
            let make = |origin| {
                Frame::new(
                    width,
                    height,
                    pixels(axis, origin, (1000, 1001), reverse, 64).into(),
                )
                .unwrap()
            };
            let options = MatchOptions {
                minimum_overlap: 24,
                maximum_shift: 20,
                maximum_mean_error: 3.0,
            };
            let mut session = Session::new(axis, make(0), options).unwrap();
            for origin in [9, 18] {
                assert!(matches!(
                    session.append(make(origin), &cancel),
                    Ok(Append::Added { .. })
                ));
            }
            assert_eq!(
                session.confirmed_edges(),
                Some(if reverse { (5, 7) } else { (7, 5) })
            );
            let output = session.compose(&cancel).unwrap();
            let previous = make(18);
            let mut changed = make(27).bytes().to_vec();
            // 工具栏消失，本帧露出的正文与上一帧运动完全吻合，但与早期固定栏证据矛盾。
            for line in 0..7 {
                let (a, b) = if reverse {
                    (63 - line - 9, 63 - line)
                } else {
                    (line + 9, line)
                };
                for cross in 0..20 {
                    let index = |along| {
                        if axis == Axis::Vertical {
                            ((along * width + cross) * 4) as usize
                        } else {
                            ((cross * width + along) * 4) as usize
                        }
                    };
                    changed[index(b)..index(b) + 4]
                        .copy_from_slice(&previous.bytes()[index(a)..index(a) + 4]);
                }
            }
            let changed = Frame::new(width, height, changed.into()).unwrap();
            assert_eq!(session.append(changed, &cancel), Err(Error::ChangedEdges));
            assert_eq!(session.frame_count(), 3);
            assert_eq!(&*session.compose(&cancel).unwrap().pixels, &*output.pixels);
        }
    }
}
