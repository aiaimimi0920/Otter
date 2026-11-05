//! Switch 的声明值与静态绘制快照；本模块不拥有本地草稿、焦点或提交权限。
use crate::{surface_clip::Rect, surface_layout_tree::Placed, surface_scene::Node};
use windows::Win32::Graphics::Direct2D::{Common::*, *};
#[path = "surface_switch_visual.rs"]
mod visual;

pub(crate) const SIZE: [f32; 2] = [34., 19.];
pub(crate) fn checked(node: &Node) -> bool {
    node.props().get("value") == Some(&serde_json::Value::Bool(true))
}
pub(crate) struct Switch {
    id: String,
    track: D2D1_ROUNDED_RECT,
    thumb: D2D1_ELLIPSE,
    clip: Rect,
    checked: bool,
    disabled: bool,
}
impl Switch {
    pub(crate) fn new(placed: &Placed<'_>, clip: Rect) -> Self {
        // 与 label 的居中 span 一致；外层布局可拉伸，轨道保持固定 DIP 大小。
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
    pub(crate) fn draw(
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
        // 外层 PaintFrame 恢复画刷；每条命令自行配对裁剪，绘制热路径不分配资源。
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
