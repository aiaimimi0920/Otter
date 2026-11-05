//! 圆形序号的边框、底色和居中 DirectWrite 文本；屏幕与导出共用。
use crate::{editor::Stroke, viewport::Viewport};
use windows::{
    core::Result,
    Win32::Graphics::{
        Direct2D::{Common::*, *},
        DirectWrite::*,
    },
};
pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    viewport: Viewport,
) -> Result<()> {
    let Some(text) = &stroke.text else {
        return Ok(());
    };
    if stroke.points.len() != 2 {
        return Ok(());
    }
    let a = stroke.points[0];
    let b = stroke.points[1];
    let left = viewport.origin.x + a.x * viewport.scale;
    let top = viewport.origin.y + a.y * viewport.scale;
    let width = (b.x - a.x).abs() * viewport.scale;
    let height = (b.y - a.y).abs() * viewport.scale;
    let ellipse = D2D1_ELLIPSE {
        point: D2D_POINT_2F {
            x: left + width / 2.0,
            y: top + height / 2.0,
        },
        radiusX: width / 2.0,
        radiusY: height / 2.0,
    };
    if let Some(fill) = stroke.fill {
        super::annotation_shapes::set_color(brush, fill, stroke.paint_alpha.fill());
        target.FillEllipse(&ellipse, brush);
    }
    super::annotation_shapes::set_color(brush, stroke.color, stroke.paint_alpha.stroke());
    if stroke.visible_outline() {
        target.DrawEllipse(&ellipse, brush, stroke.width * viewport.scale, None);
    }
    // 普通文字布局含 4 px 留白；序号使用整个圆框并按中心对齐。
    let layout = super::annotation_text::layout(
        text,
        viewport.scale,
        width + 8.0 * viewport.scale,
        height + 8.0 * viewport.scale,
    )?;
    layout.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
    layout.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
    target.DrawTextLayout(
        D2D_POINT_2F { x: left, y: top },
        &layout,
        brush,
        D2D1_DRAW_TEXT_OPTIONS_CLIP,
    );
    Ok(())
}
