//! Switch declared values and static drawing snapshots; this module does not own local drafts, focus, or submission permission.
use crate::{surface_clip::Rect, surface_layout_tree::Placed, surface_scene::Node};
use windows::Win32::Graphics::Direct2D::{Common::*, *};
#[path = "surface_switch_visual.rs"]
pub mod visual;

pub const SIZE: [f32; 2] = [34., 19.];
pub fn checked(node: &Node) -> bool {
    node.props().get("value") == Some(&serde_json::Value::Bool(true))
}
pub struct Switch {
    id: String,
    track: D2D1_ROUNDED_RECT,
    thumb: D2D1_ELLIPSE,
    clip: Rect,
    checked: bool,
    disabled: bool,
}
impl Switch {
    pub fn new(placed: &Placed<'_>, clip: Rect) -> Self {
        // Match the label's centered span; outer layout may stretch while the track keeps a fixed DIP size.
        let x = placed.origin[0] + (placed.size[0] - SIZE[0]) / 2.;
        let y = placed.origin[1] + (placed.size[1] - SIZE[1]) / 2.;
        let checked = checked(placed.node);
        Self {
            id: placed.node.id().to_owned(),
            track: D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: x + 0.5,
                    top: y + 0.5,
                    right: x + 33.5,
                    bottom: y + 18.5,
                },
                radiusX: 9.,
                radiusY: 9.,
            },
            thumb: D2D1_ELLIPSE {
                point: D2D_POINT_2F {
                    x: x + if checked { 24.5 } else { 9.5 },
                    y: y + 9.5,
                },
                radiusX: 6.5,
                radiusY: 6.5,
            },
            clip,
            checked,
            disabled: placed.node.props().get("disabled") == Some(&serde_json::Value::Bool(true)),
        }
    }
    pub fn draw(
        &self,
        target: &ID2D1RenderTarget,
        brush: &ID2D1SolidColorBrush,
        value: impl Fn(&str) -> Option<bool>,
        state: crate::surface_button_visual::State<'_>,
    ) {
        if self.clip.empty() {
            return;
        }
        let checked = value(&self.id).unwrap_or(self.checked);
        let colors = visual::palette(
            checked,
            self.disabled,
            state.hover == Some(self.id.as_str()),
            state.pressed == Some(self.id.as_str()),
        );
        let mut thumb = self.thumb;
        if checked != self.checked {
            thumb.point.x += if checked { 15. } else { -15. };
        }
        let clip = D2D_RECT_F {
            left: self.clip.left,
            top: self.clip.top,
            right: self.clip.right,
            bottom: self.clip.bottom,
        };
        // The outer PaintFrame restores the brush; each command balances its own clipping, and the drawing hot path allocates no resources.
        unsafe {
            target.PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
            brush.SetColor(&colors.track);
            target.FillRoundedRectangle(&self.track, brush);
            brush.SetColor(&colors.border);
            target.DrawRoundedRectangle(&self.track, brush, 1., None);
            brush.SetColor(&colors.thumb);
            target.FillEllipse(&thumb, brush);
            target.PopAxisAlignedClip();
        }
    }
}
