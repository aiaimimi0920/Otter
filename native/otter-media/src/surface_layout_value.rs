//! Layout lengths and node-class defaults; do not conflate missing widths with explicit auto.
use crate::{
    surface_length::{Context, Length},
    surface_scene::{Kind, Node},
};

pub fn length(node: &Node, key: &str, context: Context) -> Option<f32> {
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
