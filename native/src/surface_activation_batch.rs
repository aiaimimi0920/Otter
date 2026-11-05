//! 批次独占与精确返还：没有 owner 克隆，不把单项结果当作整批完成。
use super::*;
use crate::surface_click_event::ClickEvent;
impl Activation {
    /// 仅移动原批次的 owner；调用方另持有前缀完成证据，本方法不发送事件。
    pub(crate) fn take_batch_continuation(
        &mut self,
        events: &[ClickEvent],
        revision: u64,
        next: usize,
    ) -> Option<Owner> {
        if next == 0
            || next >= events.len()
            || events
                .iter()
                .enumerate()
                .any(|(index, event)| event.attempted() != (index < next))
        {
            return None;
        }
        crate::live_relay_http::validate_batch_identity(self.owner.as_ref()?, revision, events)
            .ok()?;
        self.take_batch_owner(events)
    }
    pub fn take_batch(&mut self, events: &[ClickEvent], revision: u64) -> Option<Owner> {
        let owner = self.owner.as_ref()?;
        crate::live_relay_http::validate_batch(owner, revision, events).ok()?;
        self.take_batch_owner(events)
    }
    /// 决定复用已尝试原事件，但仍独占完整批次；不会允许重发原点击。
    pub fn take_batch_decision(
        &mut self,
        events: &[ClickEvent],
        revision: u64,
        index: usize,
        ack: &crate::surface_action_ack::Ack,
        decision: &crate::surface_confirmation::Decision,
    ) -> Option<Owner> {
        let event = events.get(index)?;
        if !event.attempted() || decision.attempted() || !decision.matches(event, ack) {
            return None;
        }
        crate::live_relay_http::validate_batch_identity(self.owner.as_ref()?, revision, events)
            .ok()?;
        self.take_batch_owner(events)
    }
    fn take_batch_owner(&mut self, events: &[ClickEvent]) -> Option<Owner> {
        let owner = self.owner.as_ref()?;
        if events.is_empty()
            || !owner.confirmed_active()
            || self.pending.is_some()
            || self.pending_event.is_some()
            || self.pending_batch.is_some()
        {
            return None;
        }
        let mut ids = Vec::new();
        ids.try_reserve_exact(events.len()).ok()?;
        ids.extend(events.iter().map(|event| event.id().to_owned()));
        self.pending_batch = Some(ids);
        self.owner.take()
    }
    /// 允许部分事件尚未尝试，因提交可能中途停止；但列表身份和顺序必须完全不变。
    pub fn restore_batch(&mut self, events: &[ClickEvent], owner: Owner) -> Result<(), Box<Owner>> {
        let matches = self.pending_batch.as_ref().is_some_and(|ids| {
            ids.len() == events.len()
                && ids
                    .iter()
                    .zip(events)
                    .all(|(id, event)| id == event.id() && event.binding() == &self.binding)
        });
        if self.owner.is_some()
            || self.pending.is_some()
            || self.pending_event.is_some()
            || !matches
            || owner.binding() != &self.binding
            || !owner.confirmed_active()
        {
            return Err(Box::new(owner));
        }
        self.owner = Some(owner);
        self.pending_batch = None;
        Ok(())
    }
}
