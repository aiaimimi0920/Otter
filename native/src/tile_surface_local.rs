//! UI-local state for handler-free Tile values; no authoritative scene changes, pixels, text forms, or network actions.
use crate::{
    surface_scene::{Kind, Node, Scene},
    tile_surface_focus::Focus,
};
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
/// Accept only actual value controls lacking corresponding handlers; fixed-size index/numeric storage retains no labels or scene copies.
pub(crate) fn source_node(node: &Node) -> Option<Value> {
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
pub(crate) fn stamp(node: &Node) -> [u8; 32] {
    let bytes = serde_json::to_vec(&node.props().get("value")).expect("JSON value encoding");
    ring::digest::digest(&ring::digest::SHA256, &bytes)
        .as_ref()
        .try_into()
        .expect("SHA256 length")
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
    fn index(&self, focus: &Focus) -> Option<usize> {
        self.rows.iter().position(|e| {
            e.focus.anchor.view.instance_id == focus.anchor.view.instance_id
                && e.focus.node == focus.node
        })
    }
    fn current(&self, index: usize, focus: &Focus) -> bool {
        let entry = &self.rows[index];
        entry.focus.same_local(focus)
            && Arc::ptr_eq(&entry.focus.anchor.snapshot, &focus.anchor.snapshot)
            && entry.focus.anchor.scroll == focus.anchor.scroll
    }
    fn refresh(
        &mut self,
        index: usize,
        focus: &Focus,
        source: Option<(&Node, Value)>,
    ) -> Option<Value> {
        if self.rows[index].focus.same_local(focus) {
            if let Some((node, source)) = source {
                let stamp = stamp(node);
                let entry = &mut Rc::make_mut(&mut self.rows)[index];
                if stamp != entry.stamp {
                    entry.value = source;
                }
                entry.source = source;
                entry.stamp = stamp;
                entry.focus = focus.clone();
                return Some(entry.value);
            }
        }
        Rc::make_mut(&mut self.rows).remove(index);
        None
    }
    pub fn value(&mut self, focus: &Focus) -> Option<Value> {
        let index = self.index(focus);
        if let Some(index) = index.filter(|&i| self.current(i, focus)) {
            return Some(self.rows[index].value);
        }
        let scene = Scene::parse(&focus.anchor.snapshot.scene).ok();
        let source = scene
            .as_ref()
            .and_then(|s| s.root().visible_node(&focus.node))
            .and_then(|node| Some((node, source_node(node)?)));
        index
            .and_then(|i| self.refresh(i, focus, source))
            .or_else(|| source.map(|(_, value)| value))
    }
    /// Share existing limits with text sessions: 64 fields per view/256 total, without evicting old fields or silently enlarging budgets.
    pub fn set(
        &mut self,
        focus: Focus,
        value: Value,
        text: (usize, usize),
    ) -> Result<(), &'static str> {
        // Validation, generation changes, and fingerprints share one parsed node without reading local override values used for presentation.
        let scene = Scene::parse(&focus.anchor.snapshot.scene).ok();
        let source = scene
            .as_ref()
            .and_then(|s| s.root().visible_node(&focus.node))
            .and_then(|node| Some((node, source_node(node)?)));
        if let Some(index) = self.index(&focus).filter(|&i| !self.current(i, &focus)) {
            self.refresh(index, &focus, source);
        }
        let (node, source) = source.ok_or("Art本地字段已失效")?;
        let valid = match (source, value) {
            (Value::Switch(_), Value::Switch(_)) => true,
            (Value::Select(_), Value::Select(index)) => {
                let options =
                    crate::surface_select::Options::parse(node).map_err(|_| "Art本地选项失效")?;
                index.is_none_or(|i| options.value_at(i).is_some())
            }
            (Value::Slider(_), Value::Slider(v)) => crate::surface_slider::Range::parse(node)
                .is_ok_and(|r| r.normalize(v).is_ok_and(|n| n == v)),
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
        let stamp = stamp(node);
        Rc::make_mut(&mut self.rows).push(Entry {
            focus,
            source,
            stamp,
            value,
        });
        Ok(())
    }
    /// Synchronize only on new paints; same-view preparation gaps retain state but allow no new input.
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_surface_local_tests.rs"]
mod tests;
