//! 布局长度与节点类默认值；不把缺失宽度和显式 auto 混为一谈。
use crate::{
    surface_length::{Context, Length},
    surface_scene::{Kind, Node},
};

pub(crate) fn length(node: &Node, key: &str, context: Context) -> Option<f32> {
    let length = node.layout_length(key);
    if length == Some(Length::Auto) {
        return None;
    }
    length.and_then(|value| value.resolve(context)).or_else(|| {
        if node.kind() == Kind::Image && matches!(key, "maxWidth" | "maxHeight") {
            return Some(context.axis);
        }
        (key == "width"
            && matches!(
                node.kind(),
                Kind::View | Kind::Row | Kind::Column | Kind::Stack | Kind::Scroll | Kind::Grid
            ))
        .then_some(context.axis)
    })
}
