use super::*;
#[test]
fn prefix_search_matches_exhaustive_small_binary_sequences() {
    for length in 1..=6 {
        for a in 0..(1 << length) {
            let text: Vec<u64> = (0..length).map(|bit| (a >> bit) & 1).collect();
            for b in 0..(1 << length) {
                let pattern: Vec<u64> = (0..length).map(|bit| (b >> bit) & 1).collect();
                for minimum in 1..=length {
                    let expected: Vec<_> = (minimum..=length)
                        .rev()
                        .filter(|&count| text[length - count..] == pattern[..count])
                        .take(2)
                        .collect();
                    assert_eq!(overlaps(&text, &pattern, minimum), expected);
                }
            }
        }
    }
}
fn frame(axis: Axis, values: &[u8]) -> Frame {
    let (width, height) = match axis {
        Axis::Vertical => (3, values.len()),
        Axis::Horizontal => (values.len(), 3),
    };
    let mut bytes = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let (line, cross) = match axis {
                Axis::Vertical => (y, x),
                Axis::Horizontal => (x, y),
            };
            bytes.extend_from_slice(&[values[line], cross as u8, 17, 255]);
        }
    }
    Frame::new(width as u32, height as u32, bytes.into()).unwrap()
}
#[test]
fn exact_offsets_on_both_axes_compose_without_seam() {
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let a = frame(axis, &[1, 2, 3, 4, 5]);
        let b = frame(axis, &[4, 5, 6, 7, 8]);
        let cancel = AtomicBool::new(false);
        assert_eq!(match_exact(&a, &b, axis, 2, &cancel), Ok(Match::Offset(3)));
        assert_eq!(match_exact(&b, &a, axis, 2, &cancel), Ok(Match::Offset(-3)));
        let result = crate::long_capture::stitch(
            axis,
            &[
                super::super::Positioned {
                    frame: a,
                    origin: 0,
                },
                super::super::Positioned {
                    frame: b,
                    origin: 3,
                },
            ],
            &cancel,
        )
        .unwrap();
        assert_eq!(
            &*result.pixels,
            &*frame(axis, &[1, 2, 3, 4, 5, 6, 7, 8]).pixels
        );
    }
}
#[test]
fn duplicate_ambiguous_and_unrelated_are_distinct() {
    let cancel = AtomicBool::new(false);
    for axis in [Axis::Vertical, Axis::Horizontal] {
        let a = frame(axis, &[1, 2, 1, 2, 1, 2]);
        let b = frame(axis, &[1, 2, 1, 2, 3, 4]);
        assert_eq!(match_exact(&a, &a, axis, 2, &cancel), Ok(Match::Duplicate));
        assert_eq!(match_exact(&a, &b, axis, 2, &cancel), Ok(Match::Ambiguous));
        let c = frame(axis, &[9, 9, 9, 9, 9, 9]);
        assert_eq!(match_exact(&a, &c, axis, 2, &cancel), Ok(Match::NoOverlap));
    }
}
#[test]
fn invalid_options_and_cancellation_are_not_matches() {
    let axis = Axis::Vertical;
    let a = frame(axis, &[1, 2, 3]);
    let b = frame(axis, &[1, 2]);
    let cancel = AtomicBool::new(false);
    for minimum in [0, 4] {
        assert_eq!(
            match_exact(&a, &a, axis, minimum, &cancel),
            Err(Error::Dimensions)
        );
    }
    assert_eq!(
        match_exact(&a, &b, axis, 1, &cancel),
        Err(Error::Dimensions)
    );
    assert_eq!(
        match_exact(&a, &a, axis, 1, &AtomicBool::new(true)),
        Err(Error::Cancelled)
    );
}
