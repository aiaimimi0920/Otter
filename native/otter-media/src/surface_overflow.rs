//! Computed overflow values matching the frontend allowlist; shared by clipping, rounded corners, and local scroll ranges.
use crate::surface_scene::{Kind, Node};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Visible,
    Hidden,
    Auto,
}
impl Mode {
    pub fn clips(self) -> bool {
        self != Self::Visible
    }
}

pub fn computed(node: &Node) -> [Mode; 2] {
    let default = if node.kind() == Kind::Scroll {
        Mode::Auto
    } else {
        Mode::Visible
    };
    let mut axes = ["overflowX", "overflowY"].map(|key| {
        match node.layout().get(key).and_then(|value| value.as_str()) {
            Some("hidden") => Mode::Hidden,
            Some("auto") => Mode::Auto,
            // Match frontend mappings: reject arbitrary CSS keywords and fall back to node defaults for invalid values.
            _ => default,
        }
    });
    // CSS Overflow 3: when one axis has a scrollable value, visible on the other axis computes to auto.
    if axes.iter().any(|axis| axis.clips()) {
        for axis in &mut axes {
            if *axis == Mode::Visible {
                *axis = Mode::Auto;
            }
        }
    }
    axes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface_scene::Scene;
    #[test]
    fn cross_axis_normalization_preserves_hidden_and_node_defaults() {
        for (kind, layout, expected) in [
            ("view", "{}", [Mode::Visible, Mode::Visible]),
            (
                "view",
                r#"{"overflowX":"hidden"}"#,
                [Mode::Hidden, Mode::Auto],
            ),
            ("view", r#"{"overflowY":"auto"}"#, [Mode::Auto, Mode::Auto]),
            (
                "view",
                r#"{"overflowX":"clip","overflowY":"scroll"}"#,
                [Mode::Visible, Mode::Visible],
            ),
            (
                "scroll",
                r#"{"overflowX":"visible","overflowY":"hidden"}"#,
                [Mode::Auto, Mode::Hidden],
            ),
        ] {
            let json = format!(r#"{{"id":"root","type":"{kind}","layout":{layout}}}"#);
            let scene = Scene::parse(json.as_bytes()).unwrap();
            assert_eq!(computed(scene.root()), expected);
        }
    }
}
