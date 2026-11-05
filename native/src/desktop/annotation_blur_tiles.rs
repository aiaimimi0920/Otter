//! 大图按固定源图网格上传；邻域覆盖设备像素足迹，按二次幂分桶复用上传。
use super::super::effect_cache::Cache;
use crate::{blur_image::BlurImage, viewport::Viewport};
use windows::{
    core::{Interface, Result},
    Win32::Graphics::Direct2D::{Common::*, *},
};
#[path = "blur_tile_padding.rs"]
mod padding;

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    image: &BlurImage,
    viewport: Viewport,
    bounds: D2D_RECT_F,
    mask: &ID2D1Geometry,
    cache: &mut Cache,
) -> Result<()> {
    let frame = image.frame();
    let origin_x = viewport.origin.x + frame.x as f32 * viewport.scale;
    let origin_y = viewport.origin.y + frame.y as f32 * viewport.scale;
    let scale_x = frame.width as f32 / image.width() as f32 * viewport.scale;
    let scale_y = frame.height as f32 / image.height() as f32 * viewport.scale;
    let left = ((bounds.left - origin_x) / scale_x).floor().max(0.0) as u32;
    let top = ((bounds.top - origin_y) / scale_y).floor().max(0.0) as u32;
    let right = ((bounds.right - origin_x) / scale_x)
        .ceil()
        .max(0.0)
        .min(image.width() as f32) as u32;
    let bottom = ((bounds.bottom - origin_y) / scale_y)
        .ceil()
        .max(0.0)
        .min(image.height() as f32) as u32;
    if left >= right || top >= bottom {
        return Ok(());
    }
    let factory = target.GetFactory()?;
    let (pad_x, pad_y) =
        padding::for_target(target, (scale_x, scale_y), (image.width(), image.height()))?;
    for y in (top / 512 * 512..bottom).step_by(512) {
        for x in (left / 512 * 512..right).step_by(512) {
            let end_x = (x + 512).min(image.width());
            let end_y = (y + 512).min(image.height());
            let dest = D2D_RECT_F {
                left: origin_x + x as f32 * scale_x,
                top: origin_y + y as f32 * scale_y,
                right: origin_x + end_x as f32 * scale_x,
                bottom: origin_y + end_y as f32 * scale_y,
            };
            let geometry = factory.CreateRectangleGeometry(&D2D_RECT_F {
                left: dest.left - 1.0,
                top: dest.top - 1.0,
                right: dest.right + 1.0,
                bottom: dest.bottom + 1.0,
            })?;
            if mask.CompareWithGeometry(&geometry, None, 0.25)? == D2D1_GEOMETRY_RELATION_DISJOINT {
                continue;
            }
            let sx = x.saturating_sub(pad_x);
            let sy = y.saturating_sub(pad_y);
            let ex = end_x.saturating_add(pad_x).min(image.width());
            let ey = end_y.saturating_add(pad_y).min(image.height());
            let bitmap = cache.blur_region(target, image, [sx, sy, ex - sx, ey - sy])?;
            // 内部块边界采用互斥的像素归属，不能让相邻 DrawBitmap 的抗锯齿边缘叠加。
            // 使用几何遮罩而非轴对齐裁剪，确保旋转后的网格仍无重叠。
            let ownership = D2D_RECT_F {
                left: dest.left - if x == 0 { 1.0 } else { 0.0 },
                top: dest.top - if y == 0 { 1.0 } else { 0.0 },
                right: dest.right + if end_x == image.width() { 1.0 } else { 0.0 },
                bottom: dest.bottom + if end_y == image.height() { 1.0 } else { 0.0 },
            };
            let _tile_layer = super::super::annotation_opacity::push_mask_mode(
                target,
                ownership,
                factory.CreateRectangleGeometry(&ownership)?.cast()?,
                D2D1_ANTIALIAS_MODE_ALIASED,
            );
            target.DrawBitmap(
                &bitmap,
                Some(&D2D_RECT_F {
                    left: origin_x + sx as f32 * scale_x,
                    top: origin_y + sy as f32 * scale_y,
                    right: origin_x + ex as f32 * scale_x,
                    bottom: origin_y + ey as f32 * scale_y,
                }),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "blur_tile_tests.rs"]
mod tests;
