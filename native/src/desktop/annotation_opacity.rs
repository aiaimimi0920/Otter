//! Single-color paths combine color alpha and overall opacity once; shapes draw fill/stroke separately before applying overall opacity.
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
            | Tool::Text
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
    // windows-rs COM structure fields use ManuallyDrop; PushLayer already holds the required references.
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
) -> windows::core::Result<Option<Layer<'a>>> {
    let opacity = opacity(stroke);
    if opacity >= 1.0 {
        return Ok(None);
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
    // The arrow base lies along the endpoint segment, with an extra half-head-width margin; layer bounds must not exceed the actual render target.
    let pad = if stroke.tool == Tool::Bezier && stroke.arrow {
        // The tangent base may lie outside the control-point bounds; conservatively cover head length and half its width.
        (stroke.width * 6.0).max(24.0) + (stroke.width * 5.0).max(16.0) / 2.0
    } else if stroke.has_straight_arrow() {
        (stroke.width * 5.0).max(16.0) / 2.0
    } else if matches!(stroke.tool, Tool::Triangle | Tool::Polygon) {
        // Sharp-corner miters may extend beyond half the stroke width; conservatively allow the maximum miter limit.
        stroke.width * 5.0
    } else {
        stroke.width / 2.0
    } * viewport.scale
        + 2.0;
    // Layer bounds use drawing coordinates, including the host crop/scale and object rotation.
    let clip = super::annotation_rotation::visible_bounds(target, 0.0)?;
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
    Ok(Some(Layer(target)))
}
