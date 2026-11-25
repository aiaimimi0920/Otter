//! Precomputed geometry for read-only determinate progress; no native child windows, animation, or polling.
use crate::{surface_clip::Rect, surface_layout_tree::Placed, surface_scene::Node};
use windows::Win32::Graphics::Direct2D::{Common::D2D_RECT_F, *};

pub fn fraction(node: &Node) -> f32 {
    let number = |key| {
        node.props()
            .get(key)
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite())
    };
    let max = number("max").filter(|value| *value > 0.).unwrap_or(1.);
    (number("value").unwrap_or(0.).max(0.).min(max) / max) as f32
}
pub struct Progress {
    shape: D2D1_ROUNDED_RECT,
    clip: Rect,
    end: f32,
}
impl Progress {
    pub fn new(placed: &Placed<'_>, clip: Rect) -> Self {
        let [x, y] = placed.origin;
        let [w, h] = placed.size;
        let radius = 3_f32.min(w / 2.).min(h / 2.);
        Self {
            shape: D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: x,
                    top: y,
                    right: x + w,
                    bottom: y + h,
                },
                radiusX: radius,
                radiusY: radius,
            },
            clip,
            end: x + w * fraction(placed.node),
        }
    }
    pub fn draw(&self, target: &ID2D1RenderTarget, brush: &ID2D1SolidColorBrush) {
        if self.clip.empty() {
            return;
        }
        let clip = D2D_RECT_F {
            left: self.clip.left,
            top: self.clip.top,
            right: self.clip.right,
            bottom: self.clip.bottom,
        };
        // The outer PaintFrame restores the original brush color; this module strictly balances its own clip stack.
        unsafe {
            target.PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            brush.SetColor(&crate::surface_control_colors::TRACK);
            target.FillRoundedRectangle(&self.shape, brush);
            if self.end > self.shape.rect.left {
                let fill = D2D_RECT_F {
                    right: self.end,
                    ..self.shape.rect
                };
                target.PushAxisAlignedClip(&fill, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                brush.SetColor(&crate::surface_control_colors::ACTIVE);
                target.FillRoundedRectangle(&self.shape, brush);
                target.PopAxisAlignedClip();
            }
            target.PopAxisAlignedClip();
        }
    }
}
