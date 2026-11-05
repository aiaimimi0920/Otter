use super::*;
fn frame(x: i64, y: i64, width: u32, height: u32, value: u8) -> SpatialFrame {
    SpatialFrame {
        x,
        y,
        frame: Frame::new(
            width,
            height,
            (0..width * height)
                .flat_map(|_| [value, value / 2, 255 - value, 0])
                .collect::<Vec<_>>()
                .into(),
        )
        .unwrap(),
    }
}
#[test]
fn four_way_expansion_preserves_earlier_pixels_and_marks_black_pixels_covered() {
    let cancel = AtomicBool::new(false);
    let frames = [
        frame(0, 0, 4, 4, 0),
        frame(-2, 0, 4, 4, 20),
        frame(2, 0, 4, 4, 40),
        frame(-2, -2, 8, 2, 60),
        frame(-2, 4, 8, 2, 80),
    ];
    let result = stitch_spatial(&frames, &cancel).unwrap();
    assert_eq!(
        (result.x, result.y, result.width, result.height),
        (-2, -2, 8, 8)
    );
    for y in 0..8usize {
        for x in 0..8usize {
            let value = if y < 2 {
                60
            } else if y >= 6 {
                80
            } else if x < 2 {
                20
            } else if x >= 6 {
                40
            } else {
                0
            };
            assert_eq!(
                &result.pixels[(y * 8 + x) * 4..(y * 8 + x + 1) * 4],
                &[value, value / 2, 255 - value, 255]
            );
        }
    }
}
#[test]
fn holes_are_cropped_and_equal_area_prefers_top_then_left() {
    let cancel = AtomicBool::new(false);
    // L 形覆盖，两种面积均为 8，选择更靠上的竖条；未采集区域不能成为黑色内容。
    let result = stitch_spatial(&[frame(0, 0, 2, 4, 11), frame(2, 2, 2, 2, 22)], &cancel).unwrap();
    assert_eq!(
        (result.x, result.y, result.width, result.height),
        (0, 0, 2, 4)
    );
    assert!(result
        .pixels
        .chunks_exact(4)
        .all(|p| p == [11, 5, 244, 255]));
    let result =
        stitch_spatial(&[frame(-4, 4, 2, 2, 11), frame(3, -2, 2, 2, 22)], &cancel).unwrap();
    assert_eq!(
        (result.x, result.y, result.width, result.height),
        (3, -2, 2, 2)
    );
    assert!(result
        .pixels
        .chunks_exact(4)
        .all(|p| p == [22, 11, 233, 255]));
}
#[test]
fn invalid_geometry_budget_and_cancellation_release_all_temporary_leases() {
    let cancel = AtomicBool::new(false);
    assert!(matches!(stitch_spatial(&[], &cancel), Err(Error::Empty)));
    for frames in [
        vec![frame(i64::MAX, 0, 2, 2, 0)],
        vec![frame(i64::MIN, 0, 2, 2, 0), frame(0, 0, 2, 2, 0)],
        vec![frame(0, 0, 2, 2, 0), frame(8192, 8192, 2, 2, 0)],
    ] {
        assert!(matches!(
            compose(&frames, &cancel, |_| panic!(
                "must reject before allocation"
            )),
            Err(Error::OutputLimit)
        ));
    }
    let frames = [frame(0, 0, 2, 2, 10)];
    let budget = crate::byte_budget::Budget::new(16);
    let occupied = budget.reserve(1).unwrap();
    assert!(matches!(
        compose(&frames, &cancel, |n| budget.reserve(n)),
        Err(Error::Budget)
    ));
    drop(occupied);
    assert!(matches!(
        compose(&frames, &cancel, |n| {
            cancel.store(true, Ordering::Release);
            budget.reserve(n)
        }),
        Err(Error::Cancelled)
    ));
    assert_eq!(budget.used(), 0);
    cancel.store(false, Ordering::Release);
    let result = compose(&frames, &cancel, |n| budget.reserve(n)).unwrap();
    assert_eq!(budget.used(), 16);
    drop(result);
    assert_eq!(budget.used(), 0);
}
#[test]
fn histogram_matches_exhaustive_rectangle_oracle_for_every_three_by_three_mask() {
    let cancel = AtomicBool::new(false);
    for mask in 0u32..512 {
        let pixels: Vec<u8> = (0..9)
            .flat_map(|bit| [0, 0, 0, if mask & (1 << bit) != 0 { 255 } else { 0 }])
            .collect();
        let mut best = (0u32, 0u32, 0u32);
        for y in 0..3u32 {
            for x in 0..3u32 {
                for height in 1..=3 - y {
                    for width in 1..=3 - x {
                        if (y..y + height)
                            .all(|row| (x..x + width).all(|col| mask & (1 << (row * 3 + col)) != 0))
                        {
                            let area = width * height;
                            if area > best.0 || (area == best.0 && (y, x) < (best.1, best.2)) {
                                best = (area, y, x);
                            }
                        }
                    }
                }
            }
        }
        let result = coverage::largest(&pixels, 3, 3, &cancel);
        if mask == 0 {
            assert_eq!(result, Err(Error::Empty));
            continue;
        }
        let (x, y, width, height) = result.unwrap();
        assert_eq!((width * height, y, x), best, "mask {mask}");
        assert!(
            (y..y + height).all(|row| (x..x + width).all(|col| mask & (1 << (row * 3 + col)) != 0))
        );
    }
}
