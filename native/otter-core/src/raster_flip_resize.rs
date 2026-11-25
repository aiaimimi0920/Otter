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
    resized_axes(
        source,
        source_size,
        output_size,
        axis == Axis::Horizontal,
        axis == Axis::Vertical,
        cancelled,
    )
}

/// Live 底图保持编辑坐标尺寸；无镜像与双轴镜像也只分配一次。
pub fn resized_axes(
    source: &Pixels,
    source_size: (u32, u32),
    output_size: (u32, u32),
    horizontal: bool,
    vertical: bool,
    cancelled: impl Fn() -> bool,
) -> Result<Pixels, Error> {
    resized_with_budget(
        source,
        source_size,
        output_size,
        horizontal,
        vertical,
        cancelled,
        pixels::reserve,
    )
}

fn resized_with_budget(
    source: &Pixels,
    source_size: (u32, u32),
    output_size: (u32, u32),
    horizontal: bool,
    vertical: bool,
    cancelled: impl Fn() -> bool,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
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
        return raster_flip::image_axes(source, source_size, horizontal, vertical, cancelled);
    }
    let mut lease = reserve(bytes).ok_or(Error::BudgetExceeded)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| Error::BudgetExceeded)?;
    if !lease.grow_to(output.capacity()) {
        return Err(Error::BudgetExceeded);
    }
    let (sw, sh) = source_size;
    let (ow, oh) = output_size;
    // 列表只是加速缓存；预算紧张时仍按原公式逐像素采样，不能阻止原本可完成的保存/冻结。
    let x_samples = match sample_columns(ow, sw, horizontal, &cancelled, reserve) {
        Ok(samples) => Some(samples),
        Err(Error::BudgetExceeded) => None,
        Err(error) => return Err(error),
    };
    let sw_bytes = sw as usize * 4;
    let ow_bytes = ow as usize * 4;
    for y in 0..oh {
        if cancelled() {
            return Err(Error::Cancelled);
        }
        // 只初始化即将写入的行，避免在下一次取消检查前清零整帧。
        let row_start = output.len();
        output.resize(row_start + ow_bytes, 0);
        let out_row = &mut output[row_start..];
        let mirrored_y = if vertical { oh - 1 - y } else { y };
        let (y0, y1, fy) = sample(mirrored_y, oh, sh);
        let wy0 = 1.0 - fy;
        let wy1 = fy;
        let row0 = &source[y0 as usize * sw_bytes..(y0 as usize + 1) * sw_bytes];
        let row1 = &source[y1 as usize * sw_bytes..(y1 as usize + 1) * sw_bytes];
        for (x, dst) in out_row.chunks_exact_mut(4).enumerate() {
            let (x0, x1, fx) = x_samples.as_ref().map_or_else(
                || column_sample(x as u32, ow, sw, horizontal),
                |samples| samples.values[x],
            );
            let wx0 = 1.0 - fx;
            let p00 = &row0[x0..x0 + 4];
            let p01 = &row0[x1..x1 + 4];
            let p10 = &row1[x0..x0 + 4];
            let p11 = &row1[x1..x1 + 4];
            for channel in 0..4 {
                let top = p00[channel] as f64 * wx0 + p01[channel] as f64 * fx;
                let bottom = p10[channel] as f64 * wx0 + p11[channel] as f64 * fx;
                dst[channel] = (top * wy0 + bottom * wy1).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    Ok(Pixels::from_reserved(output, lease))
}

type ColumnSample = (usize, usize, f64);
struct ColumnSamples {
    values: Vec<ColumnSample>,
    _lease: crate::byte_budget::Lease,
}
fn sample_columns(
    output: u32,
    source: u32,
    horizontal: bool,
    cancelled: &impl Fn() -> bool,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<ColumnSamples, Error> {
    // 水平采样索引和权重独立于行号；临时表也计入像素池，覆盖真实 Vec capacity。
    let bytes = (output as usize)
        .checked_mul(std::mem::size_of::<ColumnSample>())
        .ok_or(Error::BudgetExceeded)?;
    let mut lease = reserve(bytes).ok_or(Error::BudgetExceeded)?;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(output as usize)
        .map_err(|_| Error::BudgetExceeded)?;
    let allocated = samples
        .capacity()
        .checked_mul(std::mem::size_of::<ColumnSample>())
        .ok_or(Error::BudgetExceeded)?;
    if !lease.grow_to(allocated) {
        return Err(Error::BudgetExceeded);
    }
    for x in 0..output {
        if x.is_multiple_of(256) && cancelled() {
            return Err(Error::Cancelled);
        }
        samples.push(column_sample(x, output, source, horizontal));
    }
    if cancelled() {
        return Err(Error::Cancelled);
    }
    Ok(ColumnSamples {
        values: samples,
        _lease: lease,
    })
}

fn column_sample(x: u32, output: u32, source: u32, horizontal: bool) -> ColumnSample {
    let mirrored = if horizontal { output - 1 - x } else { x };
    let (x0, x1, fx) = sample(mirrored, output, source);
    (x0 as usize * 4, x1 as usize * 4, fx)
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
    fn scratch_pressure_preserves_resized_pixels_and_cancellation() {
        let source: Pixels = (0..16).collect::<Vec<u8>>().into();
        for horizontal in [false, true] {
            for vertical in [false, true] {
                let expected =
                    resized_axes(&source, (2, 2), (4, 2), horizontal, vertical, || false).unwrap();
                let budget = crate::byte_budget::Budget::new(32);
                let actual = resized_with_budget(
                    &source,
                    (2, 2),
                    (4, 2),
                    horizontal,
                    vertical,
                    || false,
                    |n| budget.reserve(n),
                )
                .unwrap();
                assert_eq!(actual, expected);
                assert_eq!(budget.used(), 32);
                drop(actual);
                let calls = std::cell::Cell::new(0);
                assert!(matches!(
                    resized_with_budget(
                        &source,
                        (2, 2),
                        (4, 2),
                        horizontal,
                        vertical,
                        || {
                            calls.set(calls.get() + 1);
                            calls.get() >= 2
                        },
                        |n| budget.reserve(n),
                    ),
                    Err(Error::Cancelled)
                ));
                assert_eq!(budget.used(), 0);
            }
        }
    }
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
    #[test]
    fn column_samples_account_all_scratch_and_release_on_cancel() {
        let bytes = 512 * std::mem::size_of::<ColumnSample>();
        let budget = crate::byte_budget::Budget::new(bytes);
        let occupied = budget.reserve(1).unwrap();
        assert!(matches!(
            sample_columns(512, 2, false, &|| false, |n| budget.reserve(n)),
            Err(Error::BudgetExceeded)
        ));
        drop(occupied);
        let samples = sample_columns(512, 2, false, &|| false, |n| budget.reserve(n)).unwrap();
        assert_eq!(
            budget.used(),
            samples.values.capacity() * std::mem::size_of::<ColumnSample>()
        );
        drop(samples);
        let calls = std::cell::Cell::new(0);
        assert!(matches!(
            sample_columns(
                512,
                2,
                false,
                &|| {
                    calls.set(calls.get() + 1);
                    calls.get() == 2
                },
                |n| budget.reserve(n)
            ),
            Err(Error::Cancelled)
        ));
        assert_eq!(budget.used(), 0);
    }
}
