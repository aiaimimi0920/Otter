//! 同一字段 FIFO 的本地 JS 出口；ticket 在 DOM 帧接纳后结算，不等待或伪造远端回执。
use super::*;
use crate::surface_javascript_wire::{InputPhase, KeyPhase, KeyboardInput, LocalInput, TextInput};
pub(crate) struct Prepared {
    pub id: String,
    pub input: LocalInput,
}
impl Mount {
    pub(crate) fn prepare_javascript(
        &mut self,
        presentation: &Presentation,
        binding: &Binding,
        until: Option<&Fence>,
        viewport_epoch: u64,
        pointer_active: bool,
        hover_active: bool,
    ) -> Result<Option<Prepared>> {
        let nodes = self.preparation_nodes(presentation, binding, std::time::Instant::now())?;
        // stale 清理后同次读取新队首，不能因未启动 child 而把 terminal 留待外部唤醒。
        self.reconcile_pointer_front(presentation, viewport_epoch, pointer_active, hover_active)?;
        if let Some(fence) = until {
            match self.intents.state(fence) {
                FenceState::Ready | FenceState::InFlight => return Ok(None),
                FenceState::Invalid => {
                    return Err(Error::new(E_FAIL, "JavaScript edit prefix invalidated"))
                }
                FenceState::Pending => {}
            }
        }
        let Some(intent) = self.intents.front().cloned() else {
            return Ok(None);
        };
        if let intents::Edge::Hover(hover) = &intent.edge {
            return Ok(Some(self.prepare_hover(&intent, hover)));
        }
        if let intents::Edge::Pointer(pointer) = &intent.edge {
            return self
                .prepare_pointer(
                    presentation,
                    &intent,
                    pointer,
                    viewport_epoch,
                    pointer_active,
                )
                .map(Some);
        }
        let field = self
            .fields
            .get_mut(&intent.node)
            .ok_or_else(|| Error::new(E_FAIL, "JavaScript text field unavailable"))?;
        if field.session.composing() {
            return Ok(None);
        }
        let node = nodes
            .get(intent.node.as_str())
            .filter(|_| !field.disabled)
            .ok_or_else(|| Error::new(E_FAIL, "JavaScript text field disabled"))?;
        if !node.events().is_empty() {
            return Err(Error::new(
                E_FAIL,
                "JavaScript field cannot declare remote actions",
            ));
        }
        let id = intent.local_id();
        let input = match intent.edge {
            intents::Edge::Pointer(_) | intents::Edge::Hover(_) => unreachable!("prepared above"),
            intents::Edge::Keyboard { flags, .. } => LocalInput::Keyboard(KeyboardInput {
                target: intent.node.clone(),
                flags,
                phase: if flags & 1 == 0 {
                    KeyPhase::Keydown
                } else {
                    KeyPhase::Keyup
                },
            }),
            intents::Edge::Text(phase) => LocalInput::Text(TextInput {
                target: intent.node.clone(),
                value: intent.value.clone(),
                phase: match phase {
                    Phase::Input => InputPhase::Input,
                    Phase::Change => InputPhase::Change,
                },
            }),
        };
        input
            .validate()
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid JavaScript text input"))?;
        field
            .session
            .prepare_local(node, binding, &id, &intent.value)?;
        self.intents.prepared(Some(&id));
        if self.intents.has_node(&intent.node) {
            field.session.protect_queued()?;
        }
        Ok(Some(Prepared { id, input }))
    }
    pub(crate) fn settle_javascript(&mut self, id: &str) -> Result<()> {
        // listener 可合法移除/替换字段；旧 session/ticket 已随合同销毁，不触及新 session。
        if let Some(node) = self.intents.prepared_node(id) {
            if let Some(field) = self.fields.get_mut(node) {
                field.session.settle(id)?;
                self.values.update(node, field.session.form_value());
            }
        }
        Ok(())
    }
}
