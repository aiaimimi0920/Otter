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
            // Nonperiodic fixtures; linear-modulo gradients create approximate periodicity that analyzers should classify as ambiguous.
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

#[test]
fn fast_noisy_scroll_with_coarse_samples_on_blank_lines_keeps_the_real_overlap() {
    let cancel = AtomicBool::new(false);
    let blank: Vec<_> = (0..16)
        .flat_map(|i| {
            let row = 173 + position(i, 16, 83);
            [row, if row == 255 { row - 1 } else { row + 1 }]
        })
        .collect();
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let make = |origin: u32, noise: u8| {
            let (width, height) = if axis == Axis::Vertical {
                (20, 256)
            } else {
                (256, 20)
            };
            let mut bytes = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    let (along, cross) = if axis == Axis::Vertical {
                        (y, x)
                    } else {
                        (x, y)
                    };
                    let world = along + origin;
                    let mut seed = world * 997 + cross * 7919 + 1;
                    seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
                    seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
                    let value = if blank.contains(&world) {
                        128
                    } else {
                        ((seed ^ (seed >> 16)) % 220) as u8
                    };
                    bytes.extend_from_slice(&[value + noise, value / 2 + noise, 240 - value, 255]);
                }
            }
            Frame::new(width, height, bytes.into()).unwrap()
        };
        let a = make(0, 0);
        let b = make(173, 2);
        let options = MatchOptions {
            minimum_overlap: 64,
            maximum_shift: 255,
            maximum_mean_error: 3.0,
        };
        for (previous, current, delta) in [(&a, &b, 173), (&b, &a, -173)] {
            assert_eq!(
                match_approximate(previous, current, axis, options, &cancel),
                Ok(ApproximateMatch::Offset {
                    delta,
                    mean_error: 1.0
                })
            );
        }
        let missing = make(300, 2);
        assert!(
            !matches!(
                match_approximate(&a, &missing, axis, options, &cancel),
                Ok(ApproximateMatch::Offset { .. })
            ),
            "missing overlap must never be fabricated"
        );
    }
}

#[test]
fn early_score_rejection_preserves_every_viable_match_and_ambiguity_margin() {
    let cancel = AtomicBool::new(false);
    let a = image(Axis::Vertical, 0, 0);
    let b = image(Axis::Vertical, 9, 2);
    let a = View::new(&a, 0, 0, 20, 48).unwrap();
    let b = View::new(&b, 0, 0, 20, 48).unwrap();
    let mut pruned = 0;
    for delta in -24..=24 {
        let full = score(
            a,
            b,
            Axis::Vertical,
            delta,
            (16, 16),
            f64::INFINITY,
            &cancel,
        )
        .unwrap();
        let fast = score(a, b, Axis::Vertical, delta, (16, 16), 5.0, &cancel).unwrap();
        if fast.error.is_infinite() {
            assert!(full.error > 5.0 && full.texture >= 4.0);
            pruned += 1;
        } else {
            assert_eq!((fast.error, fast.texture), (full.error, full.texture));
        }
    }
    assert!(pruned > 0);
}

#[test]
fn blank_page_margins_do_not_hide_a_unique_textured_match() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let make = |origin: u32, noise: u8| {
            let (width, height) = if axis == Axis::Vertical {
                (20, 256)
            } else {
                (256, 20)
            };
            let mut bytes = Vec::new();
            for y in 0..height {
                for x in 0..width {
                    let (along, cross) = if axis == Axis::Vertical {
                        (y, x)
                    } else {
                        (x, y)
                    };
                    let world = along + origin;
                    let mut seed = world * 997 + cross * 7919 + 1;
                    seed = (seed ^ (seed >> 16)).wrapping_mul(0x7feb352d);
                    seed = (seed ^ (seed >> 15)).wrapping_mul(0x846ca68b);
                    let value = if world < 100 {
                        100
                    } else if world >= 156 {
                        106
                    } else {
                        ((seed ^ (seed >> 16)) % 220) as u8
                    };
                    bytes.extend_from_slice(&[value + noise, value / 2 + noise, 240 - value, 255]);
                }
            }
            Frame::new(width, height, bytes.into()).unwrap()
        };
        let a = make(0, 0);
        let b = make(9, 2);
        let options = MatchOptions {
            minimum_overlap: 64,
            maximum_shift: 255,
            maximum_mean_error: 3.0,
        };
        for (previous, current, delta) in [(&a, &b, 9), (&b, &a, -9)] {
            assert_eq!(
                match_approximate(previous, current, axis, options, &AtomicBool::new(false)),
                Ok(ApproximateMatch::Offset {
                    delta,
                    mean_error: 1.0
                })
            );
        }
    }
}
