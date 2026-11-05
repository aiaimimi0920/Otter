//! 绘制预计算模糊快照；遮罩和白色覆盖共用同一图层，不在绘制回调执行卷积。
use crate::{editor::Stroke, viewport::Viewport};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::E_INVALIDARG,
        Graphics::Direct2D::{Common::*, *},
    },
};
#[path = "annotation_blur_geometry.rs"]
mod geometry;
#[cfg(test)]
#[path = "blur_local_tile_tests.rs"]
mod local_tile_tests;
#[path = "annotation_blur_tiles.rs"]
mod tiles;
pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    stroke: &Stroke,
    viewport: Viewport,
    cache: &mut super::effect_cache::Cache,
) -> Result<()> {
    draw_impl(target, stroke, viewport, cache, None)
}

// 测试可以在相同遮罩/裁切下强制整图绘制作对照，生产仍按纹理尺寸自动分块。
unsafe fn draw_impl(
    target: &ID2D1RenderTarget,
    stroke: &Stroke,
    viewport: Viewport,
    cache: &mut super::effect_cache::Cache,
    tiled: Option<bool>,
) -> Result<()> {
    let image = stroke
        .blur
        .as_ref()
        .ok_or_else(|| Error::new(E_INVALIDARG, "Blur image missing"))?;
    let mask = geometry::mask(target, stroke, viewport, image)?;
    let mut bounds = mask.GetBounds(None)?;
    let visible = super::annotation_rotation::visible_bounds(target, 0.0)?;
    bounds.left = bounds.left.max(visible.0);
    bounds.top = bounds.top.max(visible.1);
    bounds.right = bounds.right.min(visible.2);
    bounds.bottom = bounds.bottom.min(visible.3);
    if bounds.right <= bounds.left || bounds.bottom <= bounds.top {
        return Ok(());
    }
    let _layer = super::annotation_opacity::push_mask(target, bounds, mask.clone());
    let _effect_clip = image
        .effect_clip()
        .map(|_| clip(target, image.frame(), viewport))
        .transpose()?;
    {
        let _source_clip = image
            .effect_clip()
            .map(|rect| clip(target, rect, viewport))
            .transpose()?;
        if tiled.unwrap_or(image.width() > 2048 || image.height() > 2048) {
            tiles::draw(target, image, viewport, bounds, &mask, cache)?;
        } else {
            let bitmap = cache.blur(target, image)?;
            let frame = image.frame();
            target.DrawBitmap(
                &bitmap,
                Some(&D2D_RECT_F {
                    left: viewport.origin.x + frame.x as f32 * viewport.scale,
                    top: viewport.origin.y + frame.y as f32 * viewport.scale,
                    right: viewport.origin.x + (frame.x + frame.width) as f32 * viewport.scale,
                    bottom: viewport.origin.y + (frame.y + frame.height) as f32 * viewport.scale,
                }),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                None,
            );
        }
    }
    let wash = target.CreateSolidColorBrush(
        &D2D1_COLOR_F {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 0.08,
        },
        None,
    )?;
    let bounds = geometry::wash_bounds(image, viewport, bounds);
    if bounds.right > bounds.left && bounds.bottom > bounds.top {
        target.FillRectangle(&bounds, &wash);
    }
    Ok(())
}

unsafe fn clip(
    target: &ID2D1RenderTarget,
    rect: crate::crop_geometry::Rect,
    viewport: Viewport,
) -> Result<super::annotation_opacity::Layer<'_>> {
    use windows::core::Interface;
    let bounds = D2D_RECT_F {
        left: viewport.origin.x + rect.x as f32 * viewport.scale,
        top: viewport.origin.y + rect.y as f32 * viewport.scale,
        right: viewport.origin.x + (rect.x + rect.width) as f32 * viewport.scale,
        bottom: viewport.origin.y + (rect.y + rect.height) as f32 * viewport.scale,
    };
    // AxisAlignedClip 会把旋转后的框扩为轴对齐包围框，不能作为 Canvas 局部 clip。
    let mask = target
        .GetFactory()?
        .CreateRectangleGeometry(&bounds)?
        .cast()?;
    Ok(super::annotation_opacity::push_mask(target, bounds, mask))
}
