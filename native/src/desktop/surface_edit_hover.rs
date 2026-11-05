//! 通用可见 DOM 命中与 Hover 的原 Mount 准备接缝；不伪造文本字段或远端动作。
use super::*;
use crate::surface_javascript_wire::{HoverInput, LocalInput};
pub(crate) fn dom_node(presentation: &Presentation, id: &str) -> bool {
    fn visible(node: &Node, id: &str) -> bool {
        node.visible()
            && (node.id() == id || node.visible_children().any(|child| visible(child, id)))
    }
    presentation
        .geometry()
        .nodes()
        .iter()
        .any(|node| node.id() == id && !node.clip().empty())
        && visible(presentation.scene().root(), id)
}
impl Mount {
    pub(crate) fn has_javascript_hovers(&self) -> bool {
        self.intents.has_hovers()
    }
    pub(crate) fn javascript_hover(&mut self, input: HoverInput, epoch: u64) -> Result<()> {
        input
            .validate()
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid hover input"))?;
        if !self.editing || self.binding.is_none() {
            return Err(Error::new(E_FAIL, "Hover input inactive"));
        }
        // 高速 move 的非终止样本可在满队列时拒绝；已经预留的离开不能被其他writer占用。
        self.intents.push_hover(intents::HoverIntent {
            input,
            viewport_epoch: epoch,
        });
        Ok(())
    }
    pub(super) fn prepare_hover(
        &mut self,
        intent: &intents::Intent,
        hover: &intents::HoverIntent,
    ) -> javascript::Prepared {
        let id = intent.local_id();
        self.intents.prepared(Some(&id));
        javascript::Prepared {
            id,
            input: LocalInput::Hover(hover.input.clone()),
        }
    }
}
