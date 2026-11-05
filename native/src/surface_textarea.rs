//! textarea 默认盒模型；测量、父绘制与 EDIT 投影共用，不修改原始声明。
use crate::{surface_length::Context, surface_scene::Node};
use serde_json::{Map, Value};
#[derive(Clone, Copy)]
pub struct BoxStyle {
    pub border: crate::surface_box::Insets,
    pub padding: crate::surface_box::Insets,
    pub radius: [f32; 2],
}
pub fn style(original: &Map<String, Value>) -> Map<String, Value> {
    let mut result = original.clone();
    for (key, value) in [
        ("color", Value::from("#f7f8ef")),
        ("background", Value::from("#111720")),
        ("borderColor", Value::from("rgba(255,255,255,0.12)")),
        ("borderWidth", Value::from(1)),
        ("borderRadius", Value::from(7)),
    ] {
        result.entry(key).or_insert(value);
    }
    result
}
pub fn padding(node: &Node, width: f32, context: Context) -> crate::surface_box::Insets {
    crate::surface_box::padding(node, width, context, [9., 6.])
}
pub fn insets(node: &Node, width: f32, context: Context) -> crate::surface_box::Insets {
    let border = crate::surface_box::borders(node, context);
    padding(node, width, context).with_border(border)
}
pub fn project(placed: &crate::surface_layout_tree::Placed<'_>, context: Context) -> BoxStyle {
    let radius = crate::surface_background::Background::new(
        placed,
        crate::surface_clip::Rect::from_box(placed.origin, placed.size),
        context,
    )
    .map(|b| {
        let (r, _) = b.outline();
        [r.radiusX, r.radiusY]
    })
    .unwrap_or([0.; 2]);
    BoxStyle {
        border: crate::surface_box::borders(placed.node, context),
        padding: padding(placed.node, placed.containing_width, context),
        radius,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textarea_box_defaults_and_relative_padding_share_layout_context() {
        let context = Context {
            axis: 100.,
            viewport: [800., 600.],
            container: [200., 100.],
            root_font: 16.,
            font: 20.,
            fraction: None,
        };
        for (padding, border, expected) in [
            (Value::Null, Value::Null, [10., 7.]),
            (Value::from("10%"), Value::from("0.5em"), [30., 30.]),
            (Value::from(0), Value::from(0), [0., 0.]),
        ] {
            let mut value = serde_json::json!({"id":"field","type":"textarea"});
            if !padding.is_null() {
                value["layout"] = serde_json::json!({"padding":padding});
            }
            if !border.is_null() {
                value["style"] = serde_json::json!({"borderWidth":border});
            }
            let scene = crate::surface_scene::Scene::parse(value.to_string().as_bytes()).unwrap();
            assert_eq!(insets(scene.root(), 200., context).start(), expected);
            assert_eq!(
                insets(scene.root(), 200., context).total(),
                expected.map(|v| 2. * v)
            );
            assert_eq!(scene.root().style()["background"], "#111720");
        }
    }
}
