//! Weak reservations in the original unique worker: stack commands still count toward 32 slots/8MiB, and revocation waits for actual owners to release budgets.
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
    /// Reservations are not a second queue; only the original worker can take commands, and taken prefixes cannot be merged/replayed meanwhile.
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
    /// Return whether result submission is still allowed; old tokens cannot release/confirm later reservations.
    pub(crate) fn release(&mut self, ticket: &Ticket) -> bool {
        let alive = self.holds(ticket);
        if self.same_ticket(ticket) {
            self.held = None;
        }
        alive
    }
    /// Restore only before HTTP attempts during brief same-identity preparation gaps; original deadlines/event IDs/order remain unchanged.
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_surface_action_pending_tests.rs"]
mod tests;
