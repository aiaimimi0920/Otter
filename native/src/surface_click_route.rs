//! 声明式 click 冒泡路径；只借用当前场景，不创建事件、授予权限或发送动作。
use crate::surface_scene::{Kind, Node, Scene, MAX_DEPTH};

/// 与当前场景同寿命的动作声明。payload 仍是不可信配置，发送端须执行协议校验。
pub struct ClickAction<'a> {
    pub node: &'a Node,
    pub action: &'a str,
}

pub struct ClickRoute<'a> {
    path: [Option<&'a Node>; MAX_DEPTH],
    remaining: usize,
}
impl<'a> ClickRoute<'a> {
    pub(crate) fn new(scene: &'a Scene, target: Option<&str>) -> Self {
        let mut route = Self {
            path: [None; MAX_DEPTH],
            remaining: 0,
        };
        if let Some(target) = target {
            route.remaining = find(scene.root(), target, &mut route.path, 0).unwrap_or(0);
        }
        route
    }
}
fn find<'a>(
    node: &'a Node,
    target: &str,
    path: &mut [Option<&'a Node>; MAX_DEPTH],
    depth: usize,
) -> Option<usize> {
    if !node.visible() || depth >= MAX_DEPTH {
        return None;
    }
    path[depth] = Some(node);
    if node.id() == target {
        return Some(depth + 1);
    }
    node.visible_children()
        .find_map(|child| find(child, target, path, depth + 1))
}
impl<'a> Iterator for ClickRoute<'a> {
    type Item = ClickAction<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        while self.remaining > 0 {
            self.remaining -= 1;
            let node = self.path[self.remaining]?;
            match node.kind() {
                Kind::Button => {
                    // 主干按钮始终阻止冒泡，包括未声明 click 的按钮。
                    self.remaining = 0;
                    if node.props().get("disabled") == Some(&serde_json::Value::Bool(true)) {
                        return None;
                    }
                }
                Kind::View | Kind::Row | Kind::Column | Kind::Stack | Kind::Scroll | Kind::Grid => {
                }
                // 文字、装饰及其他控件没有通用 click handler。
                _ => continue,
            }
            if let Some(action) = node.events().get("click") {
                return Some(ClickAction { node, action });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn images_bubble_only_to_ancestors_and_never_expose_children() {
        let scene = Scene::parse(br#"{"id":"root","type":"view","events":{"click":"outer"},"children":[{"id":"image","type":"image","events":{"click":"ignored"},"children":[{"id":"child","type":"button","events":{"click":"hidden-action"}}]}]}"#).unwrap();
        assert_eq!(actions(&scene, "image"), ["outer"]);
        assert!(actions(&scene, "child").is_empty());
    }
    fn actions(scene: &Scene, target: &str) -> Vec<String> {
        ClickRoute::new(scene, Some(target))
            .map(|entry| entry.action.to_owned())
            .collect()
    }
    #[test]
    fn containers_bubble_inner_first_but_text_click_declarations_do_not_execute() {
        let scene = Scene::parse(br#"{"id":"root","type":"view","events":{"click":"outer"},"children":[{"id":"inner","type":"column","events":{"click":"inner"},"children":[{"id":"text","type":"text","events":{"click":"ignored"}}]}]}"#).unwrap();
        assert_eq!(actions(&scene, "text"), ["inner", "outer"]);
        assert!(actions(&scene, "missing").is_empty());
    }
    #[test]
    fn buttons_stop_bubbling_even_without_an_action_or_when_disabled() {
        for props in [r#"{}"#, r#"{"disabled":true}"#, r#"{"disabled":"true"}"#] {
            let json = format!(
                r#"{{"id":"root","type":"view","events":{{"click":"outer"}},"children":[{{"id":"button","type":"button","props":{props},"events":{{"click":"button"}}}},{{"id":"empty","type":"button"}}]}}"#
            );
            let scene = Scene::parse(json.as_bytes()).unwrap();
            assert!(actions(&scene, "empty").is_empty());
            let expected = if props == r#"{"disabled":true}"# {
                vec![]
            } else {
                vec!["button"]
            };
            assert_eq!(actions(&scene, "button"), expected);
        }
    }
    #[test]
    fn hidden_subtrees_have_no_route_and_payload_is_borrowed_not_copied() {
        let scene = Scene::parse(br#"{"id":"root","type":"view","props":{"eventPayload":{"value":7}},"events":{"click":"outer"},"children":[{"id":"hidden","type":"view","props":{"visible":false},"children":[{"id":"text","type":"text"}]}]}"#).unwrap();
        assert!(actions(&scene, "text").is_empty());
        let entry = ClickRoute::new(&scene, Some("root")).next().unwrap();
        assert!(std::ptr::eq(entry.node, scene.root()));
        assert_eq!(entry.node.props()["eventPayload"]["value"], 7);
    }
}
