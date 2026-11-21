//! 删除会话调色板颜色只重置工具创建默认槽，不修改已有标注或其历史。
use crate::stroke_style::{Color, StrokeStyle};
pub(super) fn reset(mut style: StrokeStyle, serial: bool, color: Color, alpha: u8) -> StrokeStyle {
    let matches = |actual: Option<Color>, opacity: f32| {
        let opacity = if actual.is_some() {
            (opacity * 255.0).round() as u8
        } else {
            0
        };
        if alpha == 0 {
            opacity == 0
        } else {
            opacity == alpha && actual == Some(color)
        }
    };
    if matches(Some(style.color()), style.paint_alpha().stroke()) {
        style = style.with_color(StrokeStyle::default().color());
    }
    if matches(style.fill(), style.paint_alpha().fill()) {
        style = style.with_fill(serial.then_some(Color(0, 0, 0)));
    }
    style
}
