//! 四边颜色解析与既有按钮动态色接线；几何/工厂缓存仍仅由 Border 持有。
use super::*;
use crate::{
    surface_button_visual::PaintColor, surface_color::Color, surface_length::Context,
    surface_scene::Node,
};
pub(crate) fn build(
    background: &Background,
    node: &Node,
    context: Context,
    foreground: Color,
) -> Vec<(Border, PaintColor)> {
    let width = crate::surface_box::borders(node, context);
    let Some(border) = Border::new(background, width) else {
        return Vec::new();
    };
    let keys = [
        "borderTopColor",
        "borderRightColor",
        "borderBottomColor",
        "borderLeftColor",
    ];
    let color = |key| {
        node.style()
            .get(key)
            .and_then(|v| v.as_str())
            .and_then(|v| Color::parse(v, foreground))
    };
    let uniform = color("borderColor").unwrap_or(foreground);
    if keys.iter().all(|key| !node.style().contains_key(*key)) {
        return if uniform.0[3] == 0. {
            Vec::new()
        } else {
            vec![(border, PaintColor::new(node, "borderColor", uniform))]
        };
    }
    keys.into_iter()
        .enumerate()
        .filter_map(|(side, key)| {
            let value = color(key).unwrap_or(uniform);
            let size = [width.top, width.right, width.bottom, width.left][side];
            let property = if node.style().contains_key(key) {
                key
            } else {
                "borderColor"
            };
            (size > 0. && value.0[3] > 0.)
                .then(|| (border.part(side), PaintColor::new(node, property, value)))
        })
        .collect()
}
