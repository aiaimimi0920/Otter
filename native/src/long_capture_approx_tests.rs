use super::*;
#[test]
fn dense_verification_rejects_changes_between_coarse_sample_lines() {
    let axis = Axis::Vertical;
    let a = image(axis, 0, 0);
    let b = image(axis, 9, 2);
    let mut bytes = b.pixels.to_vec();
    let sampled: Vec<_> = (0..16).map(|row| position(row, 16, 39)).collect();
    for row in 0..39 {
        if !sampled.contains(&row) {
            for byte in &mut bytes[row as usize * 80..(row as usize + 1) * 80] {
                *byte = 255 - *byte;
            }
        }
    }
    let changed = Frame::new(20, 48, bytes.into()).unwrap();
    assert_eq!(
        match_approximate(&a, &changed, axis, options(), &AtomicBool::new(false)),
        Ok(ApproximateMatch::NoOverlap)
    );
}
fn image(axis: Axis, origin: u32, noise: u8) -> Frame {
    let (width, height) = match axis {
        Axis::Vertical => (20, 48),
        Axis::Horizontal => (48, 20),
    };
    let mut bytes = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let (along, cross) = match axis {
                Axis::Vertical => (y, x),
                Axis::Horizontal => (x, y),
            };
            // 非周期夹具；线性取模渐变会产生近似周期，应被分析器判为歧义。
            let mut seed = (along + origin) * 997 + cross * 7919 + 1;
            seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
            seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
            let value = ((seed ^ (seed >> 16)) % 220) as u8;
            bytes.extend_from_slice(&[value + noise, value / 2 + noise, 240 - value, 255]);
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
fn noisy_frames_find_forward_reverse_and_zero_displacement() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let a = image(axis, 0, 0);
        for (origin, expected) in [(9, 9), (0, 0)] {
            let b = image(axis, origin, 2);
            for (first, second, delta) in [(&a, &b, expected), (&b, &a, -expected)] {
                assert!(
                    matches!(match_approximate(first, second, axis, options(), &AtomicBool::new(false)),
                    Ok(ApproximateMatch::Offset { delta: actual, mean_error }) if actual == delta && mean_error <= 1.0)
                );
            }
        }
    }
}
#[test]
fn flat_frames_and_periodic_patterns_do_not_force_a_displacement() {
    let flat = Frame::new(20, 48, vec![100; 20 * 48 * 4].into()).unwrap();
    assert_eq!(
        match_approximate(
            &flat,
            &flat,
            Axis::Vertical,
            options(),
            &AtomicBool::new(false)
        ),
        Ok(ApproximateMatch::LowTexture)
    );
    let bytes: Vec<_> = (0..48)
        .flat_map(|y| vec![if y % 2 == 0 { 10 } else { 240 }; 20 * 4])
        .collect();
    let periodic = Frame::new(20, 48, bytes.into()).unwrap();
    assert_eq!(
        match_approximate(
            &periodic,
            &periodic,
            Axis::Vertical,
            options(),
            &AtomicBool::new(false)
        ),
        Ok(ApproximateMatch::Ambiguous)
    );
}
#[test]
fn admission_cancellation_and_search_limit_are_enforced() {
    let a = image(Axis::Vertical, 0, 0);
    let b = image(Axis::Vertical, 9, 2);
    let cancel = AtomicBool::new(false);
    let mut config = options();
    config.maximum_shift = 3;
    assert_eq!(
        match_approximate(&a, &b, Axis::Vertical, config, &cancel),
        Ok(ApproximateMatch::NoOverlap)
    );
    for error in [f64::NAN, f64::INFINITY, -1.0, 13.0] {
        config.maximum_mean_error = error;
        assert_eq!(
            match_approximate(&a, &b, Axis::Vertical, config, &cancel),
            Err(Error::Dimensions)
        );
    }
    assert_eq!(
        match_approximate(&a, &b, Axis::Vertical, options(), &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
}
