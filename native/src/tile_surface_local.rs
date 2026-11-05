//! Tile无handler值的UI本地状态；不改权威scene、不持像素，不参与文本表单或网络动作。
use crate::{surface_scene::Kind, tile_surface_focus::Focus};
use std::{rc::Rc, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Value {
    Switch(bool),
    Select(Option<usize>),
    Slider(f64),
}
#[derive(Clone)]
pub(crate) struct Entry {
    pub focus: Focus,
    pub source: Value,
    pub stamp: [u8; 32],
    pub value: Value,
}
#[derive(Default)]
pub(crate) struct Cache {
    rows: Rc<Vec<Entry>>,
}
/// 只接受无对应handler的真实值控件；索引/数值存储固定大小，不复制标签/整份Art资源。
pub(crate) fn source(focus: &Focus) -> Option<Value> {
    let scene = crate::surface_scene::Scene::parse(&focus.anchor.snapshot.scene).ok()?;
    let node = scene.root().visible_node(&focus.node)?;
    source_node(node)
}
pub(crate) fn source_node(node: &crate::surface_scene::Node) -> Option<Value> {
    if node.props().get("disabled") == Some(&serde_json::Value::Bool(true)) {
        return None;
    }
    match node.kind() {
        Kind::Switch if !node.events().contains_key("change") => {
            Some(Value::Switch(crate::surface_switch::checked(node)))
        }
        Kind::Select if !node.events().contains_key("change") => Some(Value::Select(
            crate::surface_select::Options::parse(node)
                .ok()?
                .source_index(),
        )),
        Kind::Slider
            if !["input", "change"]
                .iter()
                .any(|e| node.events().contains_key(*e)) =>
        {
            Some(Value::Slider(
                crate::surface_slider::Range::parse(node).ok()?.source(),
            ))
        }
        _ => None,
    }
}
pub(crate) fn stamp(node: &crate::surface_scene::Node) -> [u8; 32] {
    let bytes = serde_json::to_vec(&node.props().get("value")).expect("JSON value encoding");
    ring::digest::digest(&ring::digest::SHA256, &bytes)
        .as_ref()
        .try_into()
        .expect("SHA256 length")
}
fn source_stamp(focus: &Focus) -> Option<[u8; 32]> {
    let scene = crate::surface_scene::Scene::parse(&focus.anchor.snapshot.scene).ok()?;
    Some(stamp(scene.root().visible_node(&focus.node)?))
}
impl Cache {
    pub fn snapshot(&self) -> Rc<Vec<Entry>> {
        self.rows.clone()
    }
    pub fn counts(&self, view: &crate::tile_surface_wire::View) -> (usize, usize) {
        (
            self.rows.len(),
            self.rows
                .iter()
                .filter(|e| &e.focus.anchor.view == view)
                .count(),
        )
    }
    pub fn clear(&mut self) -> bool {
        let changed = !self.rows.is_empty();
        if changed {
            self.rows = Default::default();
        }
        changed
    }
    pub fn value(&mut self, focus: &Focus) -> Option<Value> {
        let index = self.rows.iter().position(|e| {
            e.focus.anchor.view.instance_id == focus.anchor.view.instance_id
                && e.focus.node == focus.node
        });
        if let Some(index) = index {
            let e = &self.rows[index];
            if e.focus.same_local(focus) {
                if Arc::ptr_eq(&e.focus.anchor.snapshot, &focus.anchor.snapshot)
                    && e.focus.anchor.scroll == focus.anchor.scroll
                {
                    return Some(e.value);
                }
                if let Some(source) = source(focus) {
                    let stamp = source_stamp(focus)?;
                    let e = &mut Rc::make_mut(&mut self.rows)[index];
                    if stamp != e.stamp {
                        e.value = source;
                    }
                    e.source = source;
                    e.stamp = stamp;
                    e.focus = focus.clone();
                    return Some(e.value);
                }
            }
            Rc::make_mut(&mut self.rows).remove(index);
        }
        source(focus)
    }
    /// 与文本会话合计沿用每view64/总256字段，不挤出旧字段或静默扩大预算。
    pub fn set(
        &mut self,
        focus: Focus,
        value: Value,
        text: (usize, usize),
    ) -> Result<(), &'static str> {
        let source = self
            .value(&focus)
            .and_then(|_| source(&focus))
            .ok_or("Art本地字段已失效")?;
        let valid = match (source, value) {
            (Value::Switch(_), Value::Switch(_)) => true,
            (Value::Select(_), Value::Select(index)) => {
                let scene = crate::surface_scene::Scene::parse(&focus.anchor.snapshot.scene)
                    .map_err(|_| "Art本地选项失效")?;
                let node = scene
                    .root()
                    .visible_node(&focus.node)
                    .ok_or("Art本地选项失效")?;
                let options =
                    crate::surface_select::Options::parse(node).map_err(|_| "Art本地选项失效")?;
                index.is_none_or(|i| options.value_at(i).is_some())
            }
            (Value::Slider(_), Value::Slider(v)) => {
                let scene = crate::surface_scene::Scene::parse(&focus.anchor.snapshot.scene)
                    .map_err(|_| "Art本地滑块失效")?;
                let node = scene
                    .root()
                    .visible_node(&focus.node)
                    .ok_or("Art本地滑块失效")?;
                crate::surface_slider::Range::parse(node)
                    .is_ok_and(|r| r.normalize(v).is_ok_and(|n| n == v))
            }
            _ => false,
        };
        if !valid {
            return Err("Art本地字段值无效");
        }
        if let Some(e) = Rc::make_mut(&mut self.rows)
            .iter_mut()
            .find(|e| e.focus.same_local(&focus))
        {
            e.value = value;
            return Ok(());
        }
        let (total, per_view) = self.counts(&focus.anchor.view);
        if total + text.0 >= 256 || per_view + text.1 >= 64 {
            return Err("Art本地字段预算不足");
        }
        let stamp = source_stamp(&focus).ok_or("Art本地字段源失效")?;
        Rc::make_mut(&mut self.rows).push(Entry {
            focus,
            source,
            stamp,
            value,
        });
        Ok(())
    }
    /// 新paint才同步；同view准备空隙只保留，不允许借此新输入。
    pub fn reconcile(
        &mut self,
        mut current: impl FnMut(&Focus) -> Option<Focus>,
        mut awaiting: impl FnMut(&Focus) -> bool,
    ) -> bool {
        let old = self.rows.clone();
        for focus in old.iter().map(|e| &e.focus) {
            if let Some(next) = current(focus) {
                self.value(&next);
            } else if !awaiting(focus) {
                Rc::make_mut(&mut self.rows).retain(|e| !e.focus.same(focus));
            }
        }
        !Rc::ptr_eq(&old, &self.rows)
    }
}
#[cfg(test)]
#[path = "tile_surface_local_tests.rs"]
mod tests;
