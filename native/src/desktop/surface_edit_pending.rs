//! 字段挂载待交付/会话空闲观察；只读原有界队列，不消费意图或改变组合状态。
use super::*;
impl Mount {
    pub fn pending(&self) -> bool {
        self.editing
            && self.intents.front().is_some_and(|intent| {
                matches!(intent.edge, Edge::Pointer(_) | Edge::Hover(_))
                    || self
                        .fields
                        .get(&intent.node)
                        .is_some_and(|field| !field.disabled && !field.session.composing())
                    || self
                        .switches
                        .get(&intent.node)
                        .is_some_and(|field| !field.disabled)
                    || self
                        .selects
                        .get(&intent.node)
                        .is_some_and(|field| !field.disabled && field.visible)
                    || self
                        .sliders
                        .get(&intent.node)
                        .is_some_and(|field| !field.disabled && field.visible)
            })
    }
    pub(in crate::desktop) fn session_end_idle(&self) -> bool {
        self.intents.front().is_none()
            && self.fields.values().all(|field| {
                field.edit_depth == 0 && !field.session.composing() && field.session.pending() == 0
            })
    }
}
