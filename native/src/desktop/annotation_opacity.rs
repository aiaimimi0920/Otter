//! 单色路径将颜色alpha与整体opacity一次合成；形状的填充/描边独立绘制后再合成整体opacity。
use crate::{
    editor::{Stroke, Tool},
    viewport::Viewport,
};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

pub(super) struct Layer<'a>(&'a ID2D1RenderTarget);
fn separate_fill(stroke: &Stroke) -> bool {
    matches!(
        stroke.tool,
        Tool::Rectangle
            | Tool::RoundedRectangle
            | Tool::Ellipse
            | Tool::Triangle
            | Tool::Polygon
            | Tool::Serial
    )
}
pub(super) fn stroke_alpha(stroke: &Stroke) -> f32 {
    if separate_fill(stroke) {
        stroke.paint_alpha.stroke()
    } else {
        1.0
    }
}
fn opacity(stroke: &Stroke) -> f32 {
    stroke.opacity
        * if separate_fill(stroke) {
            1.0
        } else {
            stroke.paint_alpha.stroke()
        }
}
pub(super) unsafe fn push_mask<'a>(
    target: &'a ID2D1RenderTarget,
    bounds: D2D_RECT_F,
    geometry: ID2D1Geometry,
) -> Layer<'a> {
    push_mask_mode(target, bounds, geometry, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE)
}

pub(super) unsafe fn push_mask_mode<'a>(
    target: &'a ID2D1RenderTarget,
    bounds: D2D_RECT_F,
    geometry: ID2D1Geometry,
    mode: D2D1_ANTIALIAS_MODE,
) -> Layer<'a> {
    let mut parameters = D2D1_LAYER_PARAMETERS {
        contentBounds: bounds,
        geometricMask: std::mem::ManuallyDrop::new(Some(geometry)),
        maskAntialiasMode: mode,
        opacity: 1.0,
        ..Default::default()
    };
    parameters.maskTransform.M11 = 1.0;
    parameters.maskTransform.M22 = 1.0;
    target.PushLayer(&parameters, None);
    // windows-rs 的 COM 结构字段为 ManuallyDrop；PushLayer 已持有所需引用。
    std::mem::ManuallyDrop::drop(&mut parameters.geometricMask);
    Layer(target)
}
pub(super) unsafe fn push_wash(
    target: &ID2D1RenderTarget,
    bounds: crate::highlighter_bounds::Bounds,
) -> Layer<'_> {
    let mut parameters = D2D1_LAYER_PARAMETERS {
        contentBounds: D2D_RECT_F {
            left: bounds.left,
            top: bounds.top,
            right: bounds.right,
            bottom: bounds.bottom,
        },
        opacity: 0.35,
        ..Default::default()
    };
    parameters.maskTransform.M11 = 1.0;
    parameters.maskTransform.M22 = 1.0;
    target.PushLayer(&parameters, None);
    Layer(target)
}
impl Drop for Layer<'_> {
    fn drop(&mut self) {
        unsafe {
            self.0.PopLayer();
        }
    }
}

pub(super) unsafe fn push<'a>(
    target: &'a ID2D1RenderTarget,
    stroke: &Stroke,
    viewport: Viewport,
) -> Option<Layer<'a>> {
    let opacity = opacity(stroke);
    if opacity >= 1.0 {
        return None;
    }
    let mut bounds = D2D_RECT_F {
        left: f32::INFINITY,
        top: f32::INFINITY,
        right: f32::NEG_INFINITY,
        bottom: f32::NEG_INFINITY,
    };
    for point in &stroke.points {
        let x = viewport.origin.x + point.x * viewport.scale;
        let y = viewport.origin.y + point.y * viewport.scale;
        bounds.left = bounds.left.min(x);
        bounds.right = bounds.right.max(x);
        bounds.top = bounds.top.min(y);
        bounds.bottom = bounds.bottom.max(y);
    }
    // 箭头基底在端点连线内，额外覆盖半个头宽；层范围不能超过实际 render target。
    let pad = if stroke.tool == Tool::Bezier && stroke.arrow {
        // 切线基底可能在控制点包围框外，保守覆盖头长与半头宽。
        (stroke.width * 6.0).max(24.0) + (stroke.width * 5.0).max(16.0) / 2.0
    } else if stroke.has_straight_arrow() {
        (stroke.width * 5.0).max(16.0) / 2.0
    } else if matches!(stroke.tool, Tool::Triangle | Tool::Polygon) {
        // 尖角的 miter 可能越过半线宽，按最大 miter limit 保守保留。
        stroke.width * 5.0
    } else {
        stroke.width / 2.0
    } * viewport.scale
        + 2.0;
    let size = target.GetSize();
    let center = crate::rotation::center(&stroke.points);
    let center = crate::viewport::Point {
        x: viewport.origin.x + center.x * viewport.scale,
        y: viewport.origin.y + center.y * viewport.scale,
    };
    // 目标边界先逆变换回标注空间，避免裁掉“旋转后进入画面”的部分。
    let clip = crate::rotation::bounds(
        (0.0, 0.0, size.width, size.height),
        center,
        -stroke.rotation,
    );
    bounds.left = (bounds.left - pad).clamp(clip.0, clip.2);
    bounds.top = (bounds.top - pad).clamp(clip.1, clip.3);
    bounds.right = (bounds.right + pad).clamp(clip.0, clip.2);
    bounds.bottom = (bounds.bottom + pad).clamp(clip.1, clip.3);
    let mut parameters = D2D1_LAYER_PARAMETERS {
        contentBounds: bounds,
        opacity,
        ..Default::default()
    };
    parameters.maskTransform.M11 = 1.0;
    parameters.maskTransform.M22 = 1.0;
    target.PushLayer(&parameters, None);
    Some(Layer(target))
}
