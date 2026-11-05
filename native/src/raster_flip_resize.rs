//! 栅格标注层适配可见框的像素内核；一次分配完成缩放和镜像，不构造中间整图。
use crate::{
    pixels::{self, Pixels},
    raster_flip::{self, Axis, Error},
    viewport::checked_image_bytes,
};

/// 像素中心对齐的双线性采样，直接插值预乘 BGRA，避免透明边缘颜色泄漏。
/// 这是确定性的原生采样合同，尚不声明与浏览器 Canvas 下采样逐像素等价。
pub fn resized_image(
    source: &Pixels,
    source_size: (u32, u32),
    output_size: (u32, u32),
    axis: Axis,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, Error> {
    let source_bytes =
        checked_image_bytes(source_size.0, source_size.1).ok_or(Error::InvalidInput)?;
    let bytes = checked_image_bytes(output_size.0, output_size.1).ok_or(Error::InvalidInput)?;
    if source.len() != source_bytes {
        return Err(Error::InvalidInput);
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    if source_size == output_size {
        return raster_flip::image(source, source_size, axis, cancelled);
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
    let (sw, sh) = source_size;
    let (ow, oh) = output_size;
    for y in 0..oh {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        let mirrored_y = if axis == Axis::Vertical {
            oh - 1 - y
        } else {
            y
        };
        let (y0, y1, fy) = sample(mirrored_y, oh, sh);
        for x in 0..ow {
            let mirrored_x = if axis == Axis::Horizontal {
                ow - 1 - x
            } else {
                x
            };
            let (x0, x1, fx) = sample(mirrored_x, ow, sw);
            let top_left = ((y0 * sw + x0) * 4) as usize;
            let top_right = ((y0 * sw + x1) * 4) as usize;
            let bottom_left = ((y1 * sw + x0) * 4) as usize;
            let bottom_right = ((y1 * sw + x1) * 4) as usize;
            let target = ((y * ow + x) * 4) as usize;
            for channel in 0..4 {
                let top = source[top_left + channel] as f64 * (1.0 - fx)
                    + source[top_right + channel] as f64 * fx;
                let bottom = source[bottom_left + channel] as f64 * (1.0 - fx)
                    + source[bottom_right + channel] as f64 * fx;
                output[target + channel] =
                    (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    Ok(Pixels::from_reserved(output, lease))
}

fn sample(position: u32, output: u32, source: u32) -> (u32, u32, f64) {
    let coordinate = ((position as f64 + 0.5) * source as f64 / output as f64 - 0.5)
        .clamp(0.0, (source - 1) as f64);
    let low = coordinate.floor() as u32;
    (low, (low + 1).min(source - 1), coordinate - low as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upscale_mirrors_pixel_centers_and_interpolates_premultiplied_alpha() {
        let source: Pixels = vec![0, 0, 0, 0, 100, 50, 25, 100].into();
        let output = resized_image(&source, (2, 1), (3, 1), Axis::Horizontal, || false).unwrap();
        assert_eq!(&output[..], &[100, 50, 25, 100, 50, 25, 13, 50, 0, 0, 0, 0]);
        assert_eq!(&source[..], &[0, 0, 0, 0, 100, 50, 25, 100]);
    }
    #[test]
    fn vertical_downscale_averages_pixel_centers_not_edges() {
        let source: Pixels = [0u8, 40, 80, 120]
            .into_iter()
            .flat_map(|v| [v, 0, 0, 255])
            .collect::<Vec<_>>()
            .into();
        let output = resized_image(&source, (1, 4), (1, 2), Axis::Vertical, || false).unwrap();
        assert_eq!(&output[..], &[100, 0, 0, 255, 20, 0, 0, 255]);
    }
    #[test]
    fn equal_size_is_exact_and_single_pixel_expands_without_alpha_changes() {
        let source: Pixels = vec![7, 11, 19, 23].into();
        let identity = resized_image(&source, (1, 1), (1, 1), Axis::Horizontal, || false).unwrap();
        assert!(source.same_storage(&identity));
        let output = resized_image(&source, (1, 1), (4, 3), Axis::Vertical, || false).unwrap();
        assert_eq!(&output[..], &[7, 11, 19, 23].repeat(12));
    }
    #[test]
    fn cancellation_before_during_and_after_rows_never_delivers_partial_result() {
        let source: Pixels = vec![255; 16].into();
        for stop in [1, 3, 6] {
            let calls = std::cell::Cell::new(0);
            let result = resized_image(&source, (2, 2), (4, 4), Axis::Horizontal, || {
                calls.set(calls.get() + 1);
                calls.get() >= stop
            });
            assert!(matches!(result, Err(Error::Cancelled)));
            assert_eq!(&source[..], &[255; 16]);
        }
    }
    #[test]
    fn malformed_and_unbounded_dimensions_are_rejected() {
        let source: Pixels = vec![0; 4].into();
        for (input, output) in [
            ((0, 1), (2, 2)),
            ((2, 2), (2, 2)),
            ((1, 1), (u32::MAX, 1)),
            ((1, 1), (1, 0)),
        ] {
            assert!(matches!(
                resized_image(&source, input, output, Axis::Horizontal, || false),
                Err(Error::InvalidInput)
            ));
        }
    }
}
