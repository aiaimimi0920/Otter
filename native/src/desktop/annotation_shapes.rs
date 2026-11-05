//! 填充先于边框，二者共用外层标注透明度；圆角半径按 SVG 规则限制在半边长内。
use crate::{
    editor::{Stroke, Tool},
    stroke_style::Color,
};
use windows::Win32::Graphics::Direct2D::{Common::*, *};

pub(super) unsafe fn draw(
    target: &ID2D1RenderTarget,
    brush: &ID2D1SolidColorBrush,
    stroke: &Stroke,
    rect: D2D_RECT_F,
    scale: f32,
    dash: Option<&ID2D1StrokeStyle>,
) {
    let ellipse = D2D1_ELLIPSE {
        point: D2D_POINT_2F {
            x: (rect.left + rect.right) / 2.0,
            y: (rect.top + rect.bottom) / 2.0,
        },
        radiusX: (rect.right - rect.left) / 2.0,
        radiusY: (rect.bottom - rect.top) / 2.0,
    };
    let rounded = D2D1_ROUNDED_RECT {
        rect,
        radiusX: (stroke.corner_radius * scale).min(ellipse.radiusX),
        radiusY: (stroke.corner_radius * scale).min(ellipse.radiusY),
    };
    if let Some(fill) = stroke.fill {
        set_color(brush, fill, stroke.paint_alpha.fill());
        match stroke.tool {
            Tool::Rectangle => target.FillRectangle(&rect, brush),
            Tool::RoundedRectangle => target.FillRoundedRectangle(&rounded, brush),
            Tool::Ellipse => target.FillEllipse(&ellipse, brush),
            _ => {}
        }
    }
    if !stroke.visible_outline() {
        return;
    }
    set_color(brush, stroke.color, stroke.paint_alpha.stroke());
    let width = stroke.width * scale;
    match stroke.tool {
        Tool::Rectangle => target.DrawRectangle(&rect, brush, width, dash),
        Tool::RoundedRectangle => target.DrawRoundedRectangle(&rounded, brush, width, dash),
        Tool::Ellipse => target.DrawEllipse(&ellipse, brush, width, dash),
        _ => {}
    }
}

pub(super) unsafe fn set_color(brush: &ID2D1SolidColorBrush, Color(r, g, b): Color, alpha: f32) {
    brush.SetColor(&D2D1_COLOR_F {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: alpha,
    });
}
