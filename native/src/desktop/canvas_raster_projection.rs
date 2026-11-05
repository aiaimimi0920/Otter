//! D2D只接收裁到纹理边界的有界源/目标框；空区域不回退成整图。
use super::*;

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    bitmap: &Bitmap,
    fallback: &D2D_RECT_F,
    mapping: Option<crate::image_projection::Projection>,
    viewport: Viewport,
) {
    use crate::image_projection::{Projection, Sampling};
    let Some(mapping) = mapping else {
        target.DrawBitmap(
            &bitmap.bitmap,
            Some(fallback),
            1.0,
            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
            None,
        );
        return;
    };
    let Some(Sampling::Region(mapping)) =
        Projection::clipped(mapping.source, mapping.target, bitmap.size)
    else {
        return;
    };
    let source = rect(mapping.source);
    let r = mapping.target;
    let destination = rect(crate::crop_geometry::Rect {
        x: viewport.origin.x as f64 + r.x * viewport.scale as f64,
        y: viewport.origin.y as f64 + r.y * viewport.scale as f64,
        width: r.width * viewport.scale as f64,
        height: r.height * viewport.scale as f64,
    });
    target.DrawBitmap(
        &bitmap.bitmap,
        Some(&destination),
        1.0,
        D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
        Some(&source),
    );
}
fn rect(r: crate::crop_geometry::Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: r.x as f32,
        top: r.y as f32,
        right: (r.x + r.width) as f32,
        bottom: (r.y + r.height) as f32,
    }
}
