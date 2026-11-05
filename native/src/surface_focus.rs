//! 可聚焦控件快照：跟随成功布局替换，不在键盘热路径遍历或分配整个场景。
use crate::{
    surface_geometry::Geometry,
    surface_scene::{Kind, Node, Scene},
};
use std::collections::BTreeSet;

pub(crate) fn buttons(scene: &Scene, geometry: &Geometry) -> Vec<String> {
    collect_candidates(scene, geometry, true)
}
pub(crate) fn candidates(scene: &Scene, geometry: &Geometry) -> Vec<String> {
    collect_candidates(scene, geometry, false)
}
fn collect_candidates(scene: &Scene, geometry: &Geometry, visible: bool) -> Vec<String> {
    fn collect<'a>(node: &'a Node, ids: &mut BTreeSet<&'a str>) {
        if !node.visible() {
            return;
        }
        if matches!(
            node.kind(),
            Kind::Button
                | Kind::Input
                | Kind::Textarea
                | Kind::Number
                | Kind::Switch
                | Kind::Select
                | Kind::Slider
        ) && node.props().get("disabled") != Some(&serde_json::Value::Bool(true))
        {
            ids.insert(node.id());
        }
        for child in node.visible_children() {
            collect(child, ids);
        }
    }
    let mut ids = BTreeSet::new();
    collect(scene.root(), &mut ids);
    geometry
        .nodes()
        .iter()
        .filter(|node| {
            ids.contains(node.id())
                && node.size().iter().all(|v| *v > 0.)
                && (!visible || !node.clip().empty())
        })
        .map(|node| node.id().to_owned())
        .collect()
}

pub(crate) fn next<'a>(
    buttons: &'a [String],
    current: Option<&str>,
    reverse: bool,
) -> Option<&'a str> {
    if buttons.is_empty() {
        return None;
    }
    let index = current.and_then(|id| buttons.iter().position(|item| item == id));
    let index = match (index, reverse) {
        (Some(index), true) => (index + buttons.len() - 1) % buttons.len(),
        (Some(index), false) => (index + 1) % buttons.len(),
        (None, true) => buttons.len() - 1,
        (None, false) => 0,
    };
    Some(&buttons[index])
}
