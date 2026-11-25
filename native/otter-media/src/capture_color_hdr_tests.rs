use super::*;
use crate::byte_budget::Budget;
use std::sync::atomic::Ordering;

fn encode(pixels: &[[u16; 4]]) -> Vec<u8> {
    pixels
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}
#[test]
fn pq_matches_double_precision_reference_across_luminance_range() {
    for nits in 0..=10_000 {
        let power = (nits as f64 / 10_000.0).powf(2610.0 / 16384.0);
        let expected = (((3424.0 / 4096.0 + 2413.0 / 128.0 * power)
            / (1.0 + 2392.0 / 128.0 * power))
            .powf(2523.0 / 32.0)
            * 65535.0)
            .round();
        assert!(
            (f64::from(pq(nits as f32)) - expected).abs() <= 2.0,
            "nits={nits}"
        );
    }
    assert_eq!(pq(0.0), 0);
    assert_eq!(pq(-80.0), 0);
    assert_eq!(pq(20_000.0), 65535);
}
#[test]
fn padded_fp16_rows_preserve_hdr_highlights_rgb_order_and_big_endian() {
    let mut data = encode(&[[0x3c00, 0, 0, 0], [0x3c00; 4]]);
    data.extend_from_slice(&[0xff; 8]);
    data.extend(encode(&[[0x4a40; 4], [0x7c00, 0x7e00, 0xbc00, 0]]));
    data.extend_from_slice(&[0xff; 8]);
    let input = ScRgb::new(&data, 2, 2, 24).unwrap();
    let output = input
        .to_hdr(0.01, 400.0, 1.0, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        (output.width, output.height, output.rgb16_be.len()),
        (2, 2, 24)
    );
    let codes: Vec<_> = output
        .rgb16_be
        .chunks_exact(2)
        .map(|p| u16::from_be_bytes([p[0], p[1]]))
        .collect();
    assert!(codes[0] > codes[1] && codes[1] > codes[2]);
    for code in &codes[3..6] {
        assert!(code.abs_diff(pq(80.0)) <= 1);
    }
    for code in &codes[6..9] {
        assert!(code.abs_diff(pq(1000.0)) <= 1);
    }
    assert_eq!(&codes[9..], &[0, 0, 0]);
    assert!((output.analysis.maximum_nits - 1000.0).abs() < 0.01);
    let expected_average = (0.2126 * 80.0 + 80.0 + 1000.0) / 4.0;
    assert!((output.analysis.average_nits - expected_average).abs() < 0.01);
    assert_eq!(output.mastering_maximum_nits, output.analysis.maximum_nits);
    assert_eq!(output.mastering_minimum_nits, 0.01);
}
#[test]
fn output_budget_and_cancellation_release_exact_lease_and_allow_retry() {
    let data = encode(&[[0x3c00; 4]]);
    let input = ScRgb::new(&data, 1, 1, 8).unwrap();
    let cancel = AtomicBool::new(false);
    let budget = Budget::new(6);
    assert!(matches!(
        input.convert_hdr(0.0, 1000.0, 1.0, &cancel, |_| None),
        Err(Error::Budget)
    ));
    assert!(matches!(
        input.convert_hdr(0.0, 1000.0, 1.0, &cancel, |n| {
            cancel.store(true, Ordering::Release);
            budget.reserve(n)
        }),
        Err(Error::Cancelled)
    ));
    assert_eq!(budget.used(), 0);
    cancel.store(false, Ordering::Release);
    let output = input
        .convert_hdr(f32::NAN, f32::INFINITY, f32::NAN, &cancel, |n| {
            budget.reserve(n)
        })
        .unwrap();
    assert_eq!(budget.used(), 6);
    assert_eq!(
        (output.mastering_minimum_nits, output.mastering_maximum_nits),
        (0.0, 1000.0)
    );
    let clone = output.rgb16_be.clone();
    drop(output);
    assert_eq!(budget.used(), 6);
    drop(clone);
    assert_eq!(budget.used(), 0);
}
#[test]
fn repeated_hdr_pixels_across_rows_preserve_uncached_bytes_and_metadata() {
    let a = [0x3c00, 0x3800, 0x3400, 0];
    let b = [0x4a40, 0, 0x7e00, 0];
    let data = encode(&[a, a, a, b, b, a]);
    let input = ScRgb::new(&data, 2, 3, 16).unwrap();
    let output = input
        .to_hdr(0.0, 1000.0, 1.5, &AtomicBool::new(false))
        .unwrap();
    let mut peak = 0.0f32;
    let mut sum = 0.0f64;
    for (source, actual) in data.chunks_exact(8).zip(output.rgb16_be.chunks_exact(6)) {
        let rgb = rgb(source, 1.5);
        let light = luminance(rgb);
        peak = peak.max(light);
        sum += f64::from(light);
        let expected: Vec<_> = bt2020(rgb)
            .into_iter()
            .flat_map(|v| pq(v * 80.0).to_be_bytes())
            .collect();
        assert_eq!(actual, expected);
    }
    assert_eq!(
        output.analysis.maximum_nits,
        (peak * 80.0).clamp(0.0, 10_000.0)
    );
    assert_eq!(
        output.analysis.average_nits,
        ((sum / 6.0) as f32 * 80.0).clamp(0.0, 10_000.0)
    );
}
