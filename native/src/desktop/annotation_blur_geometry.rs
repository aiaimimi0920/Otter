//! 局部效果层的几何缩放；先完整加宽笔迹，再做非等比变换，不能只缩放笔宽。
use super::*;
use crate::blur_image::BlurImage;
use windows::core::Interface;

pub(super) unsafe fn mask(
    target: &ID2D1RenderTarget,
    stroke: &Stroke,
    viewport: Viewport,
    image: &BlurImage,
) -> Result<ID2D1Geometry> {
    let mask = super::super::annotation_mosaic::mask(target, stroke, viewport)?;
    let scale = image.effect_mask_scale();
    if image.effect_clip().is_none() || scale == (1.0, 1.0) {
        return Ok(mask);
    }
    let frame = image.frame();
    let x = viewport.origin.x + frame.x as f32 * viewport.scale;
    let y = viewport.origin.y + frame.y as f32 * viewport.scale;
    let mut matrix = Default::default();
    target.GetTransform(&mut matrix);
    // 创建局部几何变换而不是修改 render target，宿主缩放/旋转只能应用一次。
    matrix.M11 = scale.0;
    matrix.M12 = 0.0;
    matrix.M21 = 0.0;
    matrix.M22 = scale.1;
    matrix.M31 = x * (1.0 - scale.0);
    matrix.M32 = y * (1.0 - scale.1);
    target
        .GetFactory()?
        .CreateTransformedGeometry(&mask, &matrix)?
        .cast()
}

pub(super) fn wash_bounds(
    image: &BlurImage,
    viewport: Viewport,
    mut bounds: D2D_RECT_F,
) -> D2D_RECT_F {
    if image.effect_clip().is_some() {
        let frame = image.frame();
        let scale = image.effect_mask_scale();
        // 最外一行/列可能只是像素尺寸 ceil 的透明填充，不属于 fillRect 逻辑范围。
        bounds.left = bounds
            .left
            .max(viewport.origin.x + frame.x as f32 * viewport.scale);
        bounds.top = bounds
            .top
            .max(viewport.origin.y + frame.y as f32 * viewport.scale);
        bounds.right = bounds.right.min(
            viewport.origin.x
                + (frame.x as f32 + frame.width.ceil() as f32 * scale.0) * viewport.scale,
        );
        bounds.bottom = bounds.bottom.min(
            viewport.origin.y
                + (frame.y as f32 + frame.height.ceil() as f32 * scale.1) * viewport.scale,
        );
    }
    bounds
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blur_wash_excludes_output_rounding_padding_and_preserves_viewport_mapping() {
        let frame = crate::crop_geometry::Rect {
            x: 2.0,
            y: 4.0,
            width: 2.0,
            height: 8.0,
        };
        let image = BlurImage::new(3, 8, vec![0; 96].into(), 1)
            .unwrap()
            .with_frame(frame)
            .unwrap()
            .with_effect_clip(frame)
            .unwrap()
            .with_effect_mask_scale((8.0 / 9.0, 1.0))
            .unwrap();
        let bounds = wash_bounds(
            &image,
            Viewport {
                origin: crate::viewport::Point { x: 10.0, y: -5.0 },
                scale: 9.0,
            },
            D2D_RECT_F {
                left: -100.0,
                top: -100.0,
                right: 200.0,
                bottom: 200.0,
            },
        );
        assert_eq!(bounds.left, 28.0);
        assert_eq!(bounds.top, 31.0);
        // ceil(2 * 4/3)=3，局部逻辑宽 2.25；wash 宽应是 2*(2/2.25)，不是整个 frame。
        assert!((bounds.right - 44.0).abs() < 1e-5);
        assert_eq!(bounds.bottom, 103.0);
    }
}
