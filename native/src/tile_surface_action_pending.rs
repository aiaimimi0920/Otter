//! 原唯一worker的弱预留：栈上命令仍计32槽/8MiB，撤销后也须等实际owner释放预算。
use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Weak,
};

pub(crate) struct Ticket(Arc<AtomicBool>);
pub(super) struct Held {
    token: Weak<AtomicBool>,
    view: View,
    bytes: usize,
    cancel: bool,
    edit: Option<Weak<crate::tile_surface_edit::Edit>>,
}
impl Queue {
    pub(super) fn held_cost(&self) -> (usize, usize) {
        self.held
            .as_ref()
            .filter(|h| h.token.strong_count() > 0)
            .map_or((0, 0), |h| (1, h.bytes))
    }
    /// 预留不是第二队列；只有原worker能取命令，期间不能合并/重放已取前缀。
    pub(crate) fn reserve(
        &mut self,
        state: &State,
        now: Instant,
        ready: bool,
    ) -> Option<(Command, Ticket)> {
        if self.held_cost().0 != 0 {
            return None;
        }
        self.held = None;
        let command = if ready {
            self.take(state, now)
        } else {
            self.take_cancel(state, now)
        }?;
        let ticket = Ticket(Arc::new(AtomicBool::new(true)));
        self.held = Some(Held {
            token: Arc::downgrade(&ticket.0),
            view: command.anchor.view.clone(),
            bytes: command.bytes(),
            cancel: matches!(command.kind, Kind::Cancel { .. }),
            edit: command.edit.as_ref().map(Arc::downgrade),
        });
        Some((command, ticket))
    }
    fn same_ticket(&self, ticket: &Ticket) -> bool {
        self.held
            .as_ref()
            .is_some_and(|h| h.token.ptr_eq(&Arc::downgrade(&ticket.0)))
    }
    pub(crate) fn holds(&self, ticket: &Ticket) -> bool {
        self.same_ticket(ticket) && ticket.0.load(Ordering::Acquire)
    }
    /// 返回是否仍有权提交结果；旧token不能释放/确认后来预留。
    pub(crate) fn release(&mut self, ticket: &Ticket) -> bool {
        let alive = self.holds(ticket);
        if self.same_ticket(ticket) {
            self.held = None;
        }
        alive
    }
    /// 仅HTTP尝试前、同身份的短暂准备空隙可恢复；原期限/事件ID/顺序不变。
    pub(crate) fn defer(&mut self, command: Command, ticket: &Ticket) {
        if self.release(ticket) {
            self.pending.push_front(command);
        }
    }
    pub(super) fn retire_dispatch(&mut self, instance: Option<&str>, keep_cancel: bool) {
        if let Some(h) = self.held.as_ref().filter(|h| {
            instance.is_none_or(|id| h.view.instance_id == id) && !(keep_cancel && h.cancel)
        }) {
            if let Some(token) = h.token.upgrade() {
                token.store(false, Ordering::Release);
            }
            if let Some(edit) = h.edit.as_ref().and_then(Weak::upgrade) {
                edit.stop();
            }
        }
    }
    pub(crate) fn retire_edit_dispatch(&mut self, edit: &crate::tile_surface_edit::Edit) {
        if let Some(h) = self.held.as_ref().filter(|h| {
            !h.cancel
                && h.edit
                    .as_ref()
                    .and_then(Weak::upgrade)
                    .is_some_and(|e| e.same_group(edit))
        }) {
            if let Some(token) = h.token.upgrade() {
                token.store(false, Ordering::Release);
            }
        }
    }
}
#[cfg(test)]
#[path = "tile_surface_action_pending_tests.rs"]
mod tests;
