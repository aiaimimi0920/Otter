//! 独立画框中的标注纹理擦除；路径输入仍是文档源坐标，不提前舍入或缩成单轴比例。
use super::*;
use crate::crop_geometry::Rect;

pub fn apply_in_frame(
    source: &Pixels,
    size: (u32, u32),
    points: &[Point],
    width: f32,
    frame: Rect,
    cancelled: impl FnMut() -> bool,
) -> Result<Pixels, EraseError> {
    if !width.is_finite() || !(4.0..=96.0).contains(&width) {
        return Err(EraseError::InvalidInput);
    }
    apply_history_in_frame(source, size, points, width, frame, cancelled)
}

/// 历史笔宽会随贴图缩放，不能套用交互工具 4–96 的滑块范围。
/// 单点按 Canvas 合同保留至少半个逻辑像素的半径。
pub fn apply_history_in_frame(
    source: &Pixels,
    size: (u32, u32),
    points: &[Point],
    width: f32,
    frame: Rect,
    cancelled: impl FnMut() -> bool,
) -> Result<Pixels, EraseError> {
    apply_mapped(
        source,
        size,
        points,
        width,
        crate::image_projection::Projection {
            source: Rect {
                x: 0.0,
                y: 0.0,
                width: size.0 as f64,
                height: size.1 as f64,
            },
            target: frame,
        },
        cancelled,
    )
}

/// 工作纹理子区域的逆映射；保留全纹理以支持reset，不扩大交互笔宽或内存预算。
pub fn apply_in_projection(
    source: &Pixels,
    size: (u32, u32),
    points: &[Point],
    width: f32,
    mapping: crate::image_projection::Projection,
    cancelled: impl FnMut() -> bool,
) -> Result<Pixels, EraseError> {
    if !width.is_finite() || !(4.0..=96.0).contains(&width) {
        return Err(EraseError::InvalidInput);
    }
    apply_mapped(source, size, points, width, mapping, cancelled)
}

fn apply_mapped(
    source: &Pixels,
    size: (u32, u32),
    points: &[Point],
    width: f32,
    mapping: crate::image_projection::Projection,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Pixels, EraseError> {
    let frame = mapping.target;
    if checked_image_bytes(size.0, size.1) != Some(source.len())
        || points.is_empty()
        || points.len() > 65_536
        || !points.iter().copied().all(valid)
        || !width.is_finite()
        || width <= 0.0
        || width > 1_000_000.0
        || !frame.x.is_finite()
        || !frame.y.is_finite()
        || frame.x.abs() > 1_000_000.0
        || frame.y.abs() > 1_000_000.0
        || !frame.width.is_finite()
        || !frame.height.is_finite()
        || frame.width <= 0.0
        || frame.height <= 0.0
        || frame.width > 1_000_000.0
        || frame.height > 1_000_000.0
        || [
            mapping.source.x,
            mapping.source.y,
            mapping.source.width,
            mapping.source.height,
            mapping.source.x + mapping.source.width,
            mapping.source.y + mapping.source.height,
        ]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 1_000_000.0)
        || mapping.source.width <= 0.0
        || mapping.source.height <= 0.0
    {
        return Err(EraseError::InvalidInput);
    }
    if cancelled() {
        return Err(EraseError::Cancelled);
    }
    let width = if points.len() == 1 {
        width.max(1.0)
    } else {
        width
    };
    if frame.x == 0.0
        && frame.y == 0.0
        && frame.width == size.0 as f64
        && frame.height == size.1 as f64
        && mapping.source == frame
    {
        return apply_scaled(source, size, points, width, (1.0, 1.0), cancelled);
    }
    let scale = (
        mapping.source.width / frame.width,
        mapping.source.height / frame.height,
    );
    if !scale.0.is_finite() || !scale.1.is_finite() {
        return Err(EraseError::InvalidInput);
    }
    let (mut mapped, _lease) = allocate::<Point>(points.len())?;
    for (input, output) in points.iter().zip(&mut mapped) {
        if cancelled() {
            return Err(EraseError::Cancelled);
        }
        *output = Point {
            x: (mapping.source.x + (input.x as f64 - frame.x) * scale.0) as f32,
            y: (mapping.source.y + (input.y as f64 - frame.y) * scale.1) as f32,
        };
        if !valid(*output) {
            return Err(EraseError::InvalidInput);
        }
    }
    apply_scaled(source, size, &mapped, width, scale, cancelled)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn historical_scaled_width_does_not_relax_interactive_tool_range() {
        let source: Pixels = vec![255; 16 * 16 * 4].into();
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: 16.0,
            height: 16.0,
        };
        let points = [Point { x: 8.0, y: 8.0 }];
        for width in [0.25, 256.0] {
            assert_eq!(
                apply_in_frame(&source, (16, 16), &points, width, frame, || false),
                Err(EraseError::InvalidInput)
            );
            assert_eq!(
                apply(&source, (16, 16), &points, width, || false),
                Err(EraseError::InvalidInput)
            );
            assert!(
                apply_history_in_frame(&source, (16, 16), &points, width, frame, || false).is_ok()
            );
        }
        let small =
            apply_history_in_frame(&source, (16, 16), &points, 0.25, frame, || false).unwrap();
        let minimum =
            apply_history_in_frame(&source, (16, 16), &points, 1.0, frame, || false).unwrap();
        assert_eq!(small, minimum);
    }
    #[test]
    fn anisotropic_offset_frame_matches_independent_logical_subpixel_reference() {
        let source: Pixels = [50, 75, 100, 200].repeat(32 * 16).into();
        let frame = Rect {
            x: 10.0,
            y: -4.0,
            width: 16.0,
            height: 4.0,
        };
        let center = Point { x: 18.0, y: -2.0 };
        let result = apply_in_frame(&source, (32, 16), &[center], 4.0, frame, || false).unwrap();
        for y in 0..16usize {
            for x in 0..32usize {
                let mut coverage = 0;
                for sy in 0..4 {
                    for sx in 0..4 {
                        let lx =
                            frame.x + (x as f64 + (sx as f64 + 0.5) / 4.0) * frame.width / 32.0;
                        let ly =
                            frame.y + (y as f64 + (sy as f64 + 0.5) / 4.0) * frame.height / 16.0;
                        if (lx - 18.0).powi(2) + (ly + 2.0).powi(2) <= 4.0 {
                            coverage += 1;
                        }
                    }
                }
                for channel in 0..4 {
                    let at = (y * 32 + x) * 4 + channel;
                    assert_eq!(
                        result[at],
                        ((source[at] as u32 * (16 - coverage) + 8) / 16) as u8
                    );
                }
            }
        }
    }
    #[test]
    fn identity_frame_and_repeated_segments_keep_existing_union_semantics() {
        let source: Pixels = vec![255; 32 * 32 * 4].into();
        let points = [Point { x: 4.0, y: 8.0 }, Point { x: 20.0, y: 12.0 }];
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: 32.0,
            height: 32.0,
        };
        assert_eq!(
            apply_in_frame(&source, (32, 32), &points, 6.0, frame, || false).unwrap(),
            apply(&source, (32, 32), &points, 6.0, || false).unwrap()
        );
        let frame = Rect {
            x: 2.0,
            y: 3.0,
            width: 24.0,
            height: 16.0,
        };
        let once = apply_in_frame(&source, (32, 32), &points, 6.0, frame, || false).unwrap();
        let repeated = apply_in_frame(
            &source,
            (32, 32),
            &[points[0], points[1], points[0], points[1]],
            6.0,
            frame,
            || false,
        )
        .unwrap();
        assert_eq!(once, repeated);
    }
    #[test]
    fn invalid_frame_and_mapping_cancellation_do_not_modify_source() {
        let source: Pixels = vec![255; 16].into();
        let mut frame = Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 2.0,
        };
        assert!(matches!(
            apply_in_frame(&source, (2, 2), &[Point::default()], 4.0, frame, || false),
            Err(EraseError::InvalidInput)
        ));
        frame.width = 4.0;
        let mut polls = 0;
        assert!(matches!(
            apply_in_frame(&source, (2, 2), &[Point::default(); 3], 4.0, frame, || {
                polls += 1;
                polls == 3
            }),
            Err(EraseError::Cancelled)
        ));
        assert_eq!(&source[..], &[255; 16]);
    }
}
