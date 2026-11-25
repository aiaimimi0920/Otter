//! Static defaults and content boxes for declarative buttons; original props/layout/style remain unchanged, and the host still manages input permission.
use crate::{
    surface_length::Context,
    surface_scene::{Kind, Node},
};
use serde_json::{Map, Value};

pub fn style(original: &Map<String, Value>, disabled: bool) -> Map<String, Value> {
    let mut result = original.clone();
    // Map Neuro control/body/separator roles; fill only defaults and preserve explicit declarative styles.
    for (key, value) in [
        ("color", Value::from("#f7f8ef")),
        ("background", Value::from("#111720")),
        ("borderColor", Value::from("rgba(255,255,255,0.12)")),
        ("borderWidth", Value::from(1)),
        ("borderRadius", Value::from(7)),
        ("textAlign", Value::from("center")),
        ("opacity", Value::from(if disabled { 0.48 } else { 1.0 })),
    ] {
        result.entry(key).or_insert(value);
    }
    result
}
pub fn content(node: &Node) -> &str {
    if node.kind() == Kind::Icon {
        return node
            .props()
            .get("glyph")
            .and_then(Value::as_str)
            .unwrap_or("•");
    }
    node.props()
        .get(if node.kind() == Kind::Button {
            "label"
        } else {
            "text"
        })
        .and_then(Value::as_str)
        .unwrap_or("")
}
pub fn insets(node: &Node, width: f32, context: Context) -> crate::surface_box::Insets {
    if node.kind() == Kind::Textarea {
        return crate::surface_textarea::insets(node, width, context);
    }
    if node.kind() != Kind::Button {
        return crate::surface_box::inset(node, width, context);
    }
    let border = crate::surface_box::borders(node, context);
    crate::surface_box::padding(node, width, context, [12., 6.]).with_border(border)
}
pub fn minimum(node: &Node, width: f32, context: Context, axis: usize) -> f32 {
    let value = insets(node, width, context).total()[axis];
    if matches!(node.kind(), Kind::Button | Kind::Textarea)
        && axis == 1
        && crate::surface_layout_value::length(node, "minHeight", context).is_none()
    {
        value.max(30.)
    } else {
        value
    }
}
