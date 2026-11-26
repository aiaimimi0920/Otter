//! Content-erasure geometry, premultiplied alpha, unchanged identity, and failure-atomicity regressions.
use super::*;

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}
fn source() -> Pixels {
    [40, 80, 100, 128].repeat(32 * 32).into()
}

#[test]
#[ignore = "leases the global pixel budget; run alone in a fresh process"]
fn tiny_erase_preserves_small_remaining_budget() {
    let source: Pixels = vec![255; 4].into();
    let _pressure = pixels::reserve(pixels::DECODED_PIXEL_LIMIT - 4 - 16).unwrap();
    let output = apply(&source, (1, 1), &[p(0.5, 0.5)], 4.0, || false).unwrap();
    assert_eq!(&*output, &[0; 4]);
}

#[test]
#[ignore = "leases the global pixel budget; run alone in a fresh process"]
fn narrow_erase_does_not_pay_for_empty_tile_columns() {
    let source: Pixels = vec![255; 2 * 9000 * 4].into();
    let _pressure =
        pixels::reserve(pixels::DECODED_PIXEL_LIMIT - source.len() - 120 * 1024).unwrap();
    let output = apply(
        &source,
        (2, 9000),
        &[p(1.0, 0.5), p(1.0, 8999.5)],
        4.0,
        || false,
    )
    .unwrap();
    assert!(output.iter().all(|byte| *byte == 0));
}

#[test]
#[ignore = "leases the global pixel budget; run alone in a fresh process"]
fn broad_erase_with_partial_edge_tiles_still_fits_the_dense_budget() {
    let source: Pixels = vec![255; 145 * 145 * 4].into();
    let _pressure =
        pixels::reserve(pixels::DECODED_PIXEL_LIMIT - source.len() - 128 * 1024).unwrap();
    let points = [p(0.0, 24.0), p(145.0, 24.0), p(145.0, 121.0), p(0.0, 121.0)];
    let output = apply(&source, (145, 145), &points, 96.0, || false).unwrap();
    assert_eq!(&output[..4], &[0; 4]);
    assert_eq!(&output[output.len() - 4..], &[0; 4]);
}

#[test]
#[ignore = "leases the global pixel budget; run alone in a fresh process"]
fn thin_diagonal_erase_still_fits_when_a_dense_mask_cannot() {
    let source: Pixels = vec![255; 512 * 512 * 4].into();
    let _pressure =
        pixels::reserve(pixels::DECODED_PIXEL_LIMIT - 2 * source.len() - 256 * 1024).unwrap();
    let output = apply(
        &source,
        (512, 512),
        &[p(0.0, 0.0), p(512.0, 512.0)],
        4.0,
        || false,
    )
    .unwrap();
    assert_eq!(&output[..4], &[0; 4]);
    assert_eq!(&output[511 * 4..512 * 4], &[255; 4]);
}

#[test]
fn dot_erases_to_transparency_preserves_outside_and_premultiplied_edges() {
    let source = source();
    let original = source.to_vec();
    let output = apply(&source, (32, 32), &[p(16.25, 16.5)], 8.0, || false).unwrap();
    assert_eq!(&output[(16 * 32 + 16) * 4..(16 * 32 + 17) * 4], &[0; 4]);
    assert_eq!(&output[..32 * 4], &source[..32 * 4]);
    assert!(output
        .chunks_exact(4)
        .any(|pixel| pixel[3] > 0 && pixel[3] < 128));
    for pixel in output.chunks_exact(4) {
        assert!(pixel[..3].iter().all(|channel| *channel <= pixel[3]));
    }
    assert_eq!(&*source, original);
    assert!(!output.same_storage(&source));
}

#[test]
fn segment_order_repetition_and_subdivision_do_not_deepen_soft_edges() {
    let source = source();
    let a = p(4.25, 5.5);
    let b = p(26.25, 27.5);
    let reference = apply(&source, (32, 32), &[a, b], 6.0, || false).unwrap();
    for points in [vec![b, a], vec![a, a, b, a, b], vec![a, p(15.25, 16.5), b]] {
        assert_eq!(
            apply(&source, (32, 32), &points, 6.0, || false).unwrap(),
            reference
        );
    }
    // A diameter-4 line spans the image without gaps; off-image endpoints do not enlarge allocations.
    let output = apply(
        &source,
        (32, 32),
        &[p(-999_999.0, 16.0), p(999_999.0, 16.0)],
        4.0,
        || false,
    )
    .unwrap();
    assert!(output[15 * 32 * 4..17 * 32 * 4]
        .iter()
        .all(|byte| *byte == 0));
    assert_eq!(&output[..32 * 4], &source[..32 * 4]);
}

#[test]
fn empty_effect_reuses_source_identity() {
    let source = source();
    let output = apply(&source, (32, 32), &[p(-100.0, -100.0)], 4.0, || false).unwrap();
    assert!(output.same_storage(&source));
    let transparent: Pixels = vec![0; 32 * 32 * 4].into();
    assert!(
        apply(&transparent, (32, 32), &[p(16.0, 16.0)], 20.0, || false)
            .unwrap()
            .same_storage(&transparent)
    );
}

#[test]
fn content_inside_bounding_box_but_outside_stroke_keeps_original_snapshot() {
    let mut bytes = vec![0; 32 * 32 * 4];
    bytes[(31 * 32) * 4..(31 * 32 + 1) * 4].copy_from_slice(&[255; 4]);
    let source: Pixels = bytes.into();
    let result = apply(
        &source,
        (32, 32),
        &[p(0.0, 0.0), p(31.0, 31.0)],
        4.0,
        || false,
    )
    .unwrap();
    assert!(result.same_storage(&source));
}

#[test]
fn invalid_input_and_mid_operation_cancellation_never_modify_source() {
    let source = source();
    let original = source.to_vec();
    for width in [0.0, 3.0, 97.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            apply(&source, (32, 32), &[p(16.0, 16.0)], width, || false),
            Err(EraseError::InvalidInput)
        );
    }
    for points in [
        vec![],
        vec![p(f32::NAN, 0.0)],
        vec![p(1_000_001.0, 0.0)],
        vec![p(0.0, 0.0); 65_537],
    ] {
        assert_eq!(
            apply(&source, (32, 32), &points, 4.0, || false),
            Err(EraseError::InvalidInput)
        );
    }
    assert_eq!(
        apply(&source, (31, 32), &[p(16.0, 16.0)], 4.0, || false),
        Err(EraseError::InvalidInput)
    );
    let mut polls = 0;
    assert_eq!(
        apply(&source, (32, 32), &[p(16.0, 16.0)], 32.0, || {
            polls += 1;
            polls > 10
        }),
        Err(EraseError::Cancelled)
    );
    assert_eq!(&*source, original);
}
