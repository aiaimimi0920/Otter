//! 有界、可取消的预乘 BGRA 镜像，不解码/编码或合成图层，不改变颜色与 alpha。
pub use crate::raster_flip_resize::resized_image;
use crate::{
    erase_layers::Layers,
    pixels::{self, Pixels},
    viewport::checked_image_bytes,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidInput,
    Cancelled,
    BudgetExceeded,
}

pub fn image(
    source: &Pixels,
    size: (u32, u32),
    axis: Axis,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, Error> {
    image_axes(
        source,
        size,
        axis == Axis::Horizontal,
        axis == Axis::Vertical,
        cancelled,
    )
}

/// 两轴镜像在同一次分配/遍历中完成，避免同时持有源图、中间图和最终图。
pub fn image_axes(
    source: &Pixels,
    size: (u32, u32),
    horizontal: bool,
    vertical: bool,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, Error> {
    let bytes = checked_image_bytes(size.0, size.1).ok_or(Error::InvalidInput)?;
    if source.len() != bytes {
        return Err(Error::InvalidInput);
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    let horizontal = horizontal && size.0 > 1;
    let vertical = vertical && size.1 > 1;
    if !horizontal && !vertical {
        return Ok(source.clone());
    }
    let mut lease = pixels::reserve(bytes).ok_or(Error::BudgetExceeded)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| Error::BudgetExceeded)?;
    if !lease.grow_to(output.capacity()) {
        return Err(Error::BudgetExceeded);
    }
    output.resize(bytes, 0);
    let stride = size.0 as usize * 4;
    for (y, row) in output.chunks_exact_mut(stride).enumerate() {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        let source_y = if vertical { size.1 as usize - 1 - y } else { y };
        let input = &source[source_y * stride..(source_y + 1) * stride];
        if horizontal {
            for (target, original) in row.chunks_exact_mut(4).zip(input.chunks_exact(4).rev()) {
                target.copy_from_slice(original);
            }
        } else {
            row.copy_from_slice(input);
        }
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    Ok(Pixels::from_reserved(output, lease))
}

/// 两层都成功才交付候选；取消/预算失败时丢弃临时结果，源图完全不动。
/// 当前作用于完整源图，裁剪可见框的栅格适配由上层事务负责。
pub fn layers(source: &Layers, axis: Axis, cancelled: impl Fn() -> bool) -> Result<Layers, Error> {
    if !source.valid() {
        return Err(Error::InvalidInput);
    }
    let base = image(&source.base, source.size, axis, &cancelled)?;
    let annotations = source
        .annotations
        .as_ref()
        .map(|pixels| image(pixels, source.annotation_dimensions(), axis, &cancelled))
        .transpose()?;
    if cancelled() {
        return Err(Error::Cancelled);
    }
    Ok(Layers {
        annotation_size: source.annotation_size,
        size: source.size,
        base,
        annotations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fused_axes_match_sequential_flips_and_keep_noop_storage() {
        for size in [(1, 1), (1, 5), (7, 1), (3, 5), (8, 4)] {
            let bytes = (0..size.0 * size.1 * 4)
                .map(|n| (n % 251) as u8)
                .collect::<Vec<_>>();
            let source: Pixels = bytes.into();
            for horizontal in [false, true] {
                for vertical in [false, true] {
                    let result = image_axes(&source, size, horizontal, vertical, || false).unwrap();
                    let mut expected = source.clone();
                    if horizontal {
                        expected = image(&expected, size, Axis::Horizontal, || false).unwrap();
                    }
                    if vertical {
                        expected = image(&expected, size, Axis::Vertical, || false).unwrap();
                    }
                    assert_eq!(&result[..], &expected[..]);
                    if (!horizontal || size.0 == 1) && (!vertical || size.1 == 1) {
                        assert!(result.same_storage(&source));
                    }
                }
            }
        }
    }
    #[test]
    fn fused_flip_cancellation_never_publishes_partial_pixels() {
        let source: Pixels = vec![255; 8 * 8 * 4].into();
        let calls = std::cell::Cell::new(0);
        let result = image_axes(&source, (8, 8), true, true, || {
            calls.set(calls.get() + 1);
            calls.get() >= 4
        });
        assert!(matches!(result, Err(Error::Cancelled)));
        assert!(source.iter().all(|byte| *byte == 255));
        assert!(matches!(
            image_axes(&source, (8, 8), false, false, || true),
            Err(Error::Cancelled)
        ));
    }
    fn source() -> Layers {
        Layers {
            annotation_size: None,
            size: (3, 2),
            base: vec![
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                24,
            ]
            .into(),
            annotations: Some([0, 0, 0, 0, 20, 40, 60, 128, 0, 0, 0, 255].repeat(2).into()),
        }
    }
    #[test]
    fn both_axes_mirror_exact_pixel_groups_and_two_flips_restore_bytes() {
        let source = source();
        for axis in [Axis::Horizontal, Axis::Vertical] {
            let flipped = layers(&source, axis, || false).unwrap();
            let expected: Vec<u8> = match axis {
                Axis::Horizontal => [2, 1, 0, 5, 4, 3],
                Axis::Vertical => [3, 4, 5, 0, 1, 2],
            }
            .into_iter()
            .flat_map(|index| source.base[index * 4..index * 4 + 4].iter().copied())
            .collect();
            assert_eq!(&flipped.base[..], &expected[..]);
            assert!(!flipped.base.same_storage(&source.base));
            let restored = layers(&flipped, axis, || false).unwrap();
            assert_eq!(&restored.base[..], &source.base[..]);
            assert_eq!(
                &restored.annotations.unwrap()[..],
                &source.annotations.as_ref().unwrap()[..]
            );
        }
        assert_eq!(&source.base[..4], &[1, 2, 3, 4]);
    }
    #[test]
    fn cancellation_during_second_layer_keeps_original_layers_unchanged() {
        let source = source();
        let original = source.clone();
        let calls = std::cell::Cell::new(0);
        let result = layers(&source, Axis::Horizontal, || {
            calls.set(calls.get() + 1);
            calls.get() >= 6
        });
        assert!(matches!(result, Err(Error::Cancelled)));
        assert!(source.same_storage(&original));
        assert_eq!(
            &source.annotations.as_ref().unwrap()[4..8],
            &[20, 40, 60, 128]
        );
    }
    #[test]
    fn single_axis_noop_shares_pixels_and_invalid_buffers_are_rejected() {
        let pixels: Pixels = vec![10, 20, 30, 255].into();
        for axis in [Axis::Horizontal, Axis::Vertical] {
            assert!(image(&pixels, (1, 1), axis, || false)
                .unwrap()
                .same_storage(&pixels));
            assert!(matches!(
                image(&pixels, (1, 1), axis, || true),
                Err(Error::Cancelled)
            ));
            assert!(matches!(
                image(&pixels, (2, 1), axis, || false),
                Err(Error::InvalidInput)
            ));
        }
        let mut source = source();
        source.annotations = Some(vec![0; 4].into());
        assert!(matches!(
            layers(&source, Axis::Vertical, || false),
            Err(Error::InvalidInput)
        ));
    }
}
