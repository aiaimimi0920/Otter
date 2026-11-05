//! Canvas 与原 Mount/FIFO 的接缝；不伪造文本字段/Session，不保存第二套 DOM。
use super::*;
use crate::surface_javascript_wire::{LocalInput, PointerInput, PointerPhase};
pub(crate) fn canvas_node(presentation: &Presentation, id: &str) -> bool {
    presentation
        .geometry()
        .nodes()
        .iter()
        .any(|node| node.id() == id)
        && canvas_in_tree(presentation.scene().root(), id)
}
fn canvas_in_tree(node: &Node, id: &str) -> bool {
    node.visible()
        && if node.id() == id {
            node.canvas_size().is_some()
                && node.props().get("disabled") != Some(&serde_json::Value::Bool(true))
        } else {
            node.visible_children()
                .any(|child| canvas_in_tree(child, id))
        }
}
impl Mount {
    pub(super) fn reconcile_pointer_front(
        &mut self,
        presentation: &Presentation,
        epoch: u64,
        active: bool,
        hover_active: bool,
    ) -> Result<()> {
        let stale = self
            .intents
            .front()
            .is_some_and(|intent| match &intent.edge {
                intents::Edge::Pointer(pointer) => {
                    pointer.viewport_epoch != epoch
                        || (pointer.input.phase != PointerPhase::Pointercancel
                            && (!active || !canvas_node(presentation, &intent.node)))
                }
                intents::Edge::Hover(hover) => {
                    hover.viewport_epoch != epoch
                        || hover.input.target.as_ref().is_some_and(|target| {
                            !hover_active || !hover::dom_node(presentation, target)
                        })
                }
                _ => false,
            });
        if stale {
            self.cancel_javascript_pointers(epoch)?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn javascript_pointer_front_for_test(&self) -> Option<PointerPhase> {
        match &self.intents.front()?.edge {
            intents::Edge::Pointer(pointer) => Some(pointer.input.phase),
            _ => None,
        }
    }
    pub(crate) fn cancel_javascript_pointers(&mut self, epoch: u64) -> Result<()> {
        if !self.editing {
            return Ok(());
        }
        if !self.intents.cancel_pointers(epoch) || !self.intents.cancel_hovers(epoch) {
            return Err(Error::new(
                E_FAIL,
                "Canvas pointer cancellation unavailable",
            ));
        }
        Ok(())
    }
    pub(crate) fn javascript_pointer(
        &mut self,
        input: PointerInput,
        gesture: u64,
        viewport_epoch: u64,
    ) -> Result<()> {
        input
            .validate()
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid Canvas pointer input"))?;
        if !self.editing || self.binding.is_none() {
            return Err(Error::new(E_FAIL, "Canvas pointer input inactive"));
        }
        let cancel = input.phase == PointerPhase::Pointercancel;
        let intent = intents::PointerIntent {
            input,
            gesture,
            viewport_epoch,
        };
        let accepted = if cancel {
            self.intents.cancel_pointer(intent)
        } else {
            self.intents.push_pointer(intent)
        };
        if !accepted {
            return Err(Error::new(E_FAIL, "Canvas pointer queue full or stale"));
        }
        Ok(())
    }
    pub(super) fn prepare_pointer(
        &mut self,
        presentation: &Presentation,
        intent: &intents::Intent,
        pointer: &intents::PointerIntent,
        epoch: u64,
        active: bool,
    ) -> Result<javascript::Prepared> {
        let cancel = pointer.input.phase == PointerPhase::Pointercancel;
        if pointer.viewport_epoch != epoch
            || (!cancel && (!active || !canvas_node(presentation, &intent.node)))
        {
            self.intents.cancel_pointers(epoch);
            return Err(Error::new(
                E_FAIL,
                "Canvas pointer target or viewport became stale",
            ));
        }
        let id = intent.local_id();
        self.intents.prepared(Some(&id));
        Ok(javascript::Prepared {
            id,
            input: LocalInput::Pointer(pointer.input.clone()),
        })
    }
}
