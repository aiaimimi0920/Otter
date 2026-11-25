//! Select dynamic button colors only while drawing; preserve explicit-style precedence without scene parsing or text reflow.
use crate::{
    surface_color::Color,
    surface_scene::{Kind, Node},
};

#[derive(Clone, Copy, Default)]
pub struct State<'a> {
    pub focus: Option<&'a str>,
    pub hover: Option<&'a str>,
    pub pressed: Option<&'a str>,
}
pub struct PaintColor {
    normal: Color,
    dynamic: Option<(String, bool)>,
}
impl PaintColor {
    pub fn new(node: &Node, property: &str, normal: Color) -> Self {
        let enabled = node.kind() == Kind::Button
            && node.props().get("disabled") != Some(&serde_json::Value::Bool(true))
            && !node.explicit_style(property);
        Self {
            normal,
            dynamic: enabled.then(|| (node.id().to_owned(), property == "color")),
        }
    }
    pub fn resolve(&self, state: State<'_>) -> Color {
        let Some((id, text)) = &self.dynamic else {
            return self.normal;
        };
        let pressed = state.pressed == Some(id.as_str());
        if !pressed && state.hover != Some(id.as_str()) {
            return self.normal;
        }
        if *text {
            return Color([6. / 255., 8. / 255., 13. / 255., 1.]);
        }
        // Neuro yellowPressed = signal 82% + black; hover follows the mainline button accent-color role.
        let scale = if pressed { 0.82 } else { 1. };
        Color([217. / 255. * scale, scale, 56. / 255. * scale, 1.])
    }
}
