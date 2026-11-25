use super::*;
fn pixel(values: [u16; 4]) -> Vec<u8> {
    values.into_iter().flat_map(u16::to_le_bytes).collect()
}
#[test]
fn every_half_pattern_matches_independent_numeric_reference_after_sanitizing() {
    for bits in 0u16..=u16::MAX {
        let exponent = i32::from((bits >> 10) & 31);
        let fraction = f64::from(bits & 1023);
        let expected = if bits & 0x8000 != 0 || exponent == 31 {
            0.0
        } else if exponent == 0 {
            (fraction / 1024.0) * 2f64.powi(-14)
        } else {
            (1.0 + fraction / 1024.0) * 2f64.powi(exponent - 15)
        };
        assert_eq!(nonnegative_half(bits), expected as f32, "half {bits:04x}");
    }
}
#[test]
fn padded_rows_convert_rgba_half_to_opaque_bgra_and_apply_white_level() {
    let mut data = pixel([0x3c00, 0x3800, 0, 0x7e00]); // R=1, G=.5, B=0; ignore alpha.
    data.extend_from_slice(&[255; 8]);
    data.extend(pixel([0, 0x3400, 0x3c00, 0])); // R=0, G=.25, B=1。
    data.extend_from_slice(&[255; 8]);
    let input = ScRgb::new(&data, 1, 2, 16).unwrap();
    let cancel = AtomicBool::new(false);
    let converted = input.to_sdr(80.0, 1.0, &cancel).unwrap();
    assert_eq!((converted.width, converted.height), (1, 2));
    assert_eq!(&*converted.pixels, &[0, 188, 255, 255, 255, 137, 0, 255]);
    let dimmed = input.to_sdr(160.0, 1.0, &cancel).unwrap();
    assert_eq!(&dimmed.pixels[..4], &[0, 137, 188, 255]);
    assert_eq!(data[6..8], 0x7e00u16.to_le_bytes());
}
#[test]
fn analysis_and_invalid_numeric_metadata_follow_baseline_rules() {
    let mut data = pixel([0x3c00; 4]);
    data.extend(pixel([0; 4]));
    let input = ScRgb::new(&data, 2, 1, 16).unwrap();
    let cancel = AtomicBool::new(false);
    // Strictly above white + 5 nits; 80 nits is not above 75 + 5.
    for (white, gain, expected) in [
        (75.0, 1.0, false),
        (74.9, 1.0, true),
        (80.0, 2.0, true),
        (f32::NAN, 1.0, false),
        (80.0, f32::NAN, false),
        (80.0, -1.0, false),
        (635.0, 100.0, false),
        (634.0, 100.0, true),
    ] {
        assert_eq!(input.exceeds_sdr_white(white, gain, &cancel), Ok(expected));
    }
    let bright = pixel([0x7bff; 4]);
    let bright = ScRgb::new(&bright, 1, 1, 8).unwrap();
    assert_eq!(bright.exceeds_sdr_white(9995.0, 1.0, &cancel), Ok(false));
    assert_eq!(bright.exceeds_sdr_white(9994.0, 1.0, &cancel), Ok(true));
    assert_eq!(
        &*input.to_sdr(f32::NAN, 1.0, &cancel).unwrap().pixels,
        &*input.to_sdr(203.0, 1.0, &cancel).unwrap().pixels
    );
    let invalid = pixel([0xbc00, 0x7c00, 0x7e00, 0x3c00]);
    assert_eq!(
        &*ScRgb::new(&invalid, 1, 1, 8)
            .unwrap()
            .to_sdr(80.0, 1.0, &cancel)
            .unwrap()
            .pixels,
        &[0, 0, 0, 255]
    );
}
#[test]
fn malformed_layout_budget_and_cancellation_do_not_leak_output() {
    let data = pixel([0x3c00; 4]);
    for (width, height, stride) in [
        (0, 1, 8),
        (1, 0, 8),
        (1, 1, 7),
        (1, 2, usize::MAX),
        (16385, 1, 8),
        (1, 2, 8),
    ] {
        assert!(matches!(
            ScRgb::new(&data, width, height, stride),
            Err(Error::Layout)
        ));
    }
    let input = ScRgb::new(&data, 1, 1, 8).unwrap();
    let cancel = AtomicBool::new(false);
    let budget = crate::byte_budget::Budget::new(4);
    let occupied = budget.reserve(1).unwrap();
    assert!(matches!(
        input.convert(80.0, 1.0, &cancel, |n| budget.reserve(n)),
        Err(Error::Budget)
    ));
    drop(occupied);
    assert!(matches!(
        input.convert(80.0, 1.0, &cancel, |n| {
            cancel.store(true, Ordering::Release);
            budget.reserve(n)
        }),
        Err(Error::Cancelled)
    ));
    assert_eq!(budget.used(), 0);
    assert!(matches!(
        input.exceeds_sdr_white(80.0, 1.0, &cancel),
        Err(Error::Cancelled)
    ));
    cancel.store(false, Ordering::Release);
    let output = input
        .convert(80.0, 1.0, &cancel, |n| budget.reserve(n))
        .unwrap();
    assert_eq!(budget.used(), 4);
    drop(output);
    assert_eq!(budget.used(), 0);
}
#[test]
fn small_sdr_frames_skip_lut_and_large_lut_matches_every_half_pattern() {
    let cancel = AtomicBool::new(false);
    assert!(sdr_lut(SDR_LUT_MIN_PIXELS - 1, 1.75, 80.0 / 203.0, &cancel)
        .unwrap()
        .is_none());
    let data: Vec<_> = (0u16..=u16::MAX)
        .flat_map(|bits| pixel([bits, bits.rotate_left(5), bits ^ 0x3c00, 0]))
        .collect();
    let input = ScRgb::new(&data, 256, 256, 256 * 8).unwrap();
    let output = input.to_sdr(203.0, 1.75, &cancel).unwrap();
    for (source, actual) in data.chunks_exact(8).zip(output.pixels.chunks_exact(4)) {
        let [r, g, b] = rgb(source, 1.75);
        assert_eq!(
            actual,
            [
                srgb(b * (80.0 / 203.0)),
                srgb(g * (80.0 / 203.0)),
                srgb(r * (80.0 / 203.0)),
                255
            ]
        );
    }
    cancel.store(true, Ordering::Release);
    assert!(matches!(
        sdr_lut(SDR_LUT_MIN_PIXELS, 1.0, 1.0, &cancel),
        Err(Error::Cancelled)
    ));
}
