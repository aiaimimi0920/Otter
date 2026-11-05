//! 内容擦除的几何、预乘透明度、无变化身份及失败原子性回归。
use super::*;

fn p(x: f32, y: f32) -> Point {
    Point { x, y }
}
fn source() -> Pixels {
    [40, 80, 100, 128].repeat(32 * 32).into()
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
    // 一条直径为 4 的跨图线，中间没有跳点断层；两端超出图像不扩大分配。
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
