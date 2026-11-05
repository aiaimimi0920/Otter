//! 前端白名单对应的 overflow 计算值；裁剪、圆角与本地滚动范围共用。
use crate::surface_scene::{Kind, Node};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Visible,
    Hidden,
    Auto,
}
impl Mode {
    pub(crate) fn clips(self) -> bool {
        self != Self::Visible
    }
}

pub(crate) fn computed(node: &Node) -> [Mode; 2] {
    let default = if node.kind() == Kind::Scroll {
        Mode::Auto
    } else {
        Mode::Visible
    };
    let mut axes = ["overflowX", "overflowY"].map(|key| {
        match node.layout().get(key).and_then(|value| value.as_str()) {
            Some("hidden") => Mode::Hidden,
            Some("auto") => Mode::Auto,
            // 与前端映射一致：不接受任意 CSS 关键字，非法值回到节点默认值。
            _ => default,
        }
    });
    // CSS Overflow 3: 一轴为 scrollable value 时，另一轴 visible 计算成 auto。
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
