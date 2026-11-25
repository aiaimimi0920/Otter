//! 可聚焦控件快照：跟随成功布局替换，不在键盘热路径遍历或分配整个场景。
use crate::{
    surface_geometry::Geometry,
    surface_scene::{Kind, Node, Scene},
};
use std::collections::BTreeSet;

pub fn buttons(scene: &Scene, geometry: &Geometry) -> Vec<String> {
    let mut visible = Vec::new();
    visit_candidates(scene, geometry, |id, shown| {
        if shown {
            visible.push(id.to_owned());
        }
    });
    visible
}
/// Both lists use geometry order. Only navigation includes fully clipped controls.
pub fn lists(scene: &Scene, geometry: &Geometry) -> (Vec<String>, Vec<String>) {
    let mut visible = Vec::new();
    let mut navigation = Vec::new();
    visit_candidates(scene, geometry, |id, shown| {
        if shown {
            visible.push(id.to_owned());
        }
        navigation.push(id.to_owned());
    });
    (visible, navigation)
}
fn visit_candidates(scene: &Scene, geometry: &Geometry, mut visit: impl FnMut(&str, bool)) {
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
    for node in geometry.nodes() {
        if ids.contains(node.id()) && node.size().iter().all(|v| *v > 0.) {
            visit(node.id(), !node.clip().empty());
        }
    }
}

pub fn next<'a>(buttons: &'a [String], current: Option<&str>, reverse: bool) -> Option<&'a str> {
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
