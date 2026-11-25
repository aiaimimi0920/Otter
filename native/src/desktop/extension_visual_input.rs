//! Local overlay-scene hit testing and single-root text gestures; no Win32 calls, clipboard publication, or command execution.
use super::*;
use crate::{surface_click_event::Modifiers, surface_clip::Rect};
pub(super) struct Press {
    index: usize,
    node: String,
    start: [f32; 2],
    text: bool,
    moved: bool,
    double: bool,
}
pub(in crate::desktop) struct Hit {
    pub(in crate::desktop) index: usize,
    pub(in crate::desktop) node: String,
    pub(in crate::desktop) text: bool,
}
pub(in crate::desktop) enum Outcome {
    None,
    Copy(String),
    Command {
        command: String,
        input: serde_json::Value,
    },
}
impl Views {
    pub(in crate::desktop) fn identity(self: &Rc<Self>) -> usize {
        Rc::as_ptr(self) as usize
    }
    pub(in crate::desktop) fn hit(&self, point: [f32; 2]) -> Option<Hit> {
        for (index, (v, p)) in self.items.iter().enumerate().rev() {
            let [x, y, w, h] = v.bounds.resolve(self.size.get());
            let local = [point[0] - x, point[1] - y];
            if local[0] < 0. || local[1] < 0. || local[0] >= w || local[1] >= h {
                continue;
            }
            let p = p.borrow();
            let Some(node) = p.hit_test(local) else {
                continue;
            };
            let text = p
                .text()
                .items()
                .iter()
                .any(|item| item.node_id() == node && item.selectable());
            if text || p.click_route(local).next().is_some() {
                return Some(Hit {
                    index,
                    node: node.into(),
                    text,
                });
            }
        }
        None
    }
    pub(in crate::desktop) fn begin(&self, point: [f32; 2], double: bool) -> Result<bool> {
        let hit = self.hit(point);
        self.cancel(true);
        let Some(hit) = hit else { return Ok(false) };
        if hit.text {
            let (v, p) = &self.items[hit.index];
            let bounds = v.bounds.resolve(self.size.get());
            let point = [point[0] - bounds[0], point[1] - bounds[1]];
            let mut p = p.borrow_mut();
            p.begin_root_text_selection(point, double)?;
        }
        *self.press.borrow_mut() = Some(Press {
            index: hit.index,
            node: hit.node,
            start: point,
            text: hit.text,
            moved: false,
            double,
        });
        Ok(true)
    }
    pub(in crate::desktop) fn dragging(&self) -> bool {
        self.press.borrow().is_some()
    }
    pub(in crate::desktop) fn update(&self, point: [f32; 2]) -> Result<bool> {
        let mut press = self.press.borrow_mut();
        let Some(press) = press.as_mut() else {
            return Ok(false);
        };
        press.moved |=
            (point[0] - press.start[0]).abs() >= 3. || (point[1] - press.start[1]).abs() >= 3.;
        if press.text {
            let (v, p) = &self.items[press.index];
            let bounds = v.bounds.resolve(self.size.get());
            p.borrow_mut()
                .update_root_text_selection([point[0] - bounds[0], point[1] - bounds[1]], false)?;
        }
        Ok(true)
    }
    pub(in crate::desktop) fn finish(
        &self,
        point: [f32; 2],
        modifiers: Modifiers,
    ) -> Result<Outcome> {
        self.update(point)?;
        let Some(press) = self.press.borrow_mut().take() else {
            return Ok(Outcome::None);
        };
        let (v, p) = &self.items[press.index];
        let bounds = v.bounds.resolve(self.size.get());
        let local = [point[0] - bounds[0], point[1] - bounds[1]];
        let mut p = p.borrow_mut();
        let selected = press.text && p.update_root_text_selection(local, true)?;
        if selected || press.moved || press.double {
            return Ok(Outcome::None);
        }
        let same = p.hit_test(local) == Some(press.node.as_str());
        if !same {
            return Ok(Outcome::None);
        }
        if press.text && v.local_copy {
            return Ok(p
                .scene()
                .root()
                .visible_node(&press.node)
                .and_then(|n| n.props().get("text"))
                .and_then(serde_json::Value::as_str)
                .filter(|t| !t.trim().is_empty())
                .map(|t| Outcome::Copy(t.into()))
                .unwrap_or(Outcome::None));
        }
        let Some(action) = p.click_route(local).next() else {
            return Ok(Outcome::None);
        };
        let input = serde_json::json!({"surfaceEvent": {
            "attachmentId": v.attachment, "nodeId": action.node.id(), "event":"click",
            "action":action.action, "class":"discrete", "generation":v.generation,
            "baseRevision":v.revision,"modifiers":modifiers,
            "payload": action.node.props().get("eventPayload").filter(|v| !v.is_null()).cloned().unwrap_or_else(|| serde_json::json!({}))
        }});
        if serde_json::to_vec(&input).map_or(true, |b| {
            b.len() > crate::surface_click_event::MAX_EVENT_BYTES
        }) {
            return Err(windows::core::Error::new(
                E_INVALIDARG,
                "Extension surface event exceeds budget",
            ));
        }
        Ok(Outcome::Command {
            command: action.action.into(),
            input,
        })
    }
    pub(in crate::desktop) fn selected_text(&self) -> Result<Option<String>> {
        for (_, p) in &self.items {
            if let Some(text) = p.borrow().selected_text()? {
                return Ok(Some(text));
            }
        }
        Ok(None)
    }
    pub(in crate::desktop) fn cancel(&self, completed: bool) -> bool {
        let captured = self.press.borrow_mut().take().is_some();
        for (_, p) in &self.items {
            if completed {
                p.borrow_mut().clear_text_selection();
            } else {
                p.borrow_mut().cancel_text_drag();
            }
        }
        captured
    }
    /// HRGN supplements only selectable roots and declared click nodes; text blocks do not occupy full overlay bounds outside themselves.
    pub(in crate::desktop) fn rects(&self) -> std::cell::Ref<'_, [Rect]> {
        std::cell::Ref::map(self.regions.borrow(), |r| r.as_slice())
    }
    pub(super) fn build_regions(&self) -> Vec<Rect> {
        let mut rects = Vec::new();
        for (v, p) in &self.items {
            let p = p.borrow();
            let v = v.bounds.resolve(self.size.get());
            let bounds = Rect::from_box([0., 0.], [v[2], v[3]]);
            for node in p.geometry().nodes() {
                if p.text()
                    .items()
                    .iter()
                    .any(|item| item.node_id() == node.id() && item.selectable())
                    || crate::surface_click_route::ClickRoute::new(p.scene(), Some(node.id()))
                        .next()
                        .is_some()
                {
                    rects.push(translated(node.clip().intersect(bounds), v));
                }
            }
        }
        rects
    }
}
fn translated(r: Rect, v: [f32; 4]) -> Rect {
    Rect {
        left: r.left + v[0],
        top: r.top + v[1],
        right: r.right + v[0],
        bottom: r.bottom + v[1],
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "extension_visual_input_tests.rs"]
mod tests;
