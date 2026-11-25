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
fn compressed_histogram_matches_pixel_oracle_on_every_uneven_grid_mask() {
    let cancel = AtomicBool::new(false);
    let xs = [0, 1, 3, 6];
    let ys = [0, 2, 3, 7];
    for mask in 0u32..512 {
        let mut frames = Vec::new();
        let mut pixels = [[false; 6]; 7];
        for (row, y) in ys.windows(2).enumerate() {
            for (col, x) in xs.windows(2).enumerate() {
                if mask & (1 << (row * 3 + col)) != 0 {
                    frames.push(frame(
                        i64::from(x[0]),
                        i64::from(y[0]),
                        x[1] - x[0],
                        y[1] - y[0],
                        0,
                    ));
                    for line in &mut pixels[y[0] as usize..y[1] as usize] {
                        line[x[0] as usize..x[1] as usize].fill(true);
                    }
                }
            }
        }
        let mut best = (0, 0, 0);
        for y in 0..7 {
            for x in 0..6 {
                for height in 1..=7 - y {
                    for width in 1..=6 - x {
                        if pixels[y..y + height]
                            .iter()
                            .all(|row| row[x..x + width].iter().all(|p| *p))
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
        let result = coverage::largest(&frames, 0, 0, &cancel);
        if mask == 0 {
            assert_eq!(result, Err(Error::Empty));
            continue;
        }
        let (x, y, width, height) = result.unwrap();
        let (x, y, width, height) = (x as usize, y as usize, width as usize, height as usize);
        assert_eq!((width * height, y, x), best, "mask {mask}");
        assert!(pixels[y..y + height]
            .iter()
            .all(|row| row[x..x + width].iter().all(|p| *p)));
    }
}

#[test]
fn sparse_composition_only_reserves_delivered_pixels() {
    let cancel = AtomicBool::new(false);
    let frames = [frame(-5, -3, 4, 5, 10), frame(999, 1001, 2, 3, 20)];
    let budget = crate::byte_budget::Budget::new(80);
    let result = compose(&frames, &cancel, |bytes| budget.reserve(bytes)).unwrap();
    assert_eq!(
        (result.x, result.y, result.width, result.height),
        (-5, -3, 4, 5)
    );
    assert_eq!(budget.used(), 80);
    assert!(result
        .pixels
        .chunks_exact(4)
        .all(|p| p == [10, 5, 245, 255]));
}
