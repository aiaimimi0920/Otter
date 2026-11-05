//! Direct2D 自定义虚线长度以线宽为单位，转换后保持旧 SVG 的绝对图像长度。
use crate::{
    editor::{Stroke, Tool},
    stroke_style::DashPattern,
};
use windows::{core::Result, Win32::Graphics::Direct2D::*};

pub(super) unsafe fn create(
    target: &ID2D1RenderTarget,
    stroke: &Stroke,
) -> Result<Option<ID2D1StrokeStyle>> {
    let freehand = matches!(
        stroke.tool,
        Tool::Brush | Tool::Highlighter | Tool::Polyline
    );
    let pattern = if matches!(stroke.tool, Tool::Highlighter | Tool::Polyline) {
        DashPattern::Solid
    } else {
        stroke.dash
    };
    let linear = matches!(stroke.tool, Tool::Line | Tool::Arrow);
    if stroke.width <= 0.0
        || !stroke.width.is_finite()
        || (pattern == DashPattern::Solid && !freehand && !linear)
        || stroke.tool == Tool::Pan
    {
        return Ok(None);
    }
    let segments = pattern.segments();
    let mut normalized = [0.0; 4];
    for (out, value) in normalized.iter_mut().zip(segments) {
        *out = value / stroke.width;
    }
    let style = target.GetFactory()?.CreateStrokeStyle(
        &D2D1_STROKE_STYLE_PROPERTIES {
            startCap: if freehand && pattern == DashPattern::Solid {
                D2D1_CAP_STYLE_ROUND
            } else {
                D2D1_CAP_STYLE_FLAT
            },
            endCap: if freehand && pattern == DashPattern::Solid {
                D2D1_CAP_STYLE_ROUND
            } else {
                D2D1_CAP_STYLE_FLAT
            },
            dashCap: D2D1_CAP_STYLE_FLAT,
            lineJoin: if freehand || matches!(stroke.tool, Tool::Line | Tool::Arrow) {
                D2D1_LINE_JOIN_ROUND
            } else {
                D2D1_LINE_JOIN_MITER
            },
            miterLimit: 10.0,
            dashStyle: if pattern == DashPattern::Solid {
                D2D1_DASH_STYLE_SOLID
            } else {
                D2D1_DASH_STYLE_CUSTOM
            },
            dashOffset: 0.0,
        },
        if segments.is_empty() {
            None
        } else {
            Some(&normalized[..segments.len()])
        },
    )?;
    Ok(Some(style))
}
