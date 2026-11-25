//! 32-slot action queue from UI to the original serial Art worker; 64 short-lived confirmation anchors own neither pixels nor authorization leases.
use crate::{
    surface_action_ack::{Ack, Status},
    tile_surface_wire::{State, View},
};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};
pub(super) const WAIT_LIMIT: Duration = Duration::from_secs(120);
#[derive(Clone)]
pub(crate) struct Anchor {
    pub view: View,
    pub generation: u64,
    pub snapshot: Arc<crate::tile_surface_wire::Snapshot>,
    pub placement: String,
    pub pixel: [i32; 2],
    pub scroll: crate::surface_scroll::Scroll,
}
pub(crate) enum Kind {
    Event(serde_json::Value),
    Decision {
        confirmation: Box<crate::tile_surface_activity::Confirmation>,
        approved: bool,
    },
    Cancel {
        request: String,
        event: String,
    },
}
pub(crate) struct Command {
    pub anchor: Anchor,
    pub kind: Kind,
    pub until: Instant,
    pub edit: Option<Arc<crate::tile_surface_edit::Edit>>,
}
#[derive(Clone)]
pub(crate) struct Accepted {
    pub anchor: Anchor,
    pub request: String,
    pub action: String,
    pub(super) until: Instant,
    pub(super) edit: Option<Arc<crate::tile_surface_edit::Edit>>,
}
#[derive(Default)]
pub(crate) struct Queue {
    pub(super) pending: VecDeque<Command>,
    held: Option<pending::Held>,
    pub states: BTreeMap<String, Arc<State>>,
    pub accepted: BTreeMap<String, Accepted>,
    /// A locale resource ID or an unchanged machine diagnostic; resolve only at presentation.
    pub notice: &'static str,
    pub origin: String,
    pub device: String,
    // Weak in-flight identity of the original unique worker; new explicit fields cannot use new Groups to bypass same-view serial prefixes.
    pub(super) inflight: Option<(View, u64, std::sync::Weak<crate::tile_surface_edit::Edit>)>,
    #[cfg(test)]
    pub test_offered: u64,
    #[cfg(test)]
    pub test_dispatch: Option<serde_json::Value>,
}
impl Queue {
    pub fn clear(&mut self) {
        self.retire_dispatch(None, false);
        for c in &self.pending {
            if let Some(e) = &c.edit {
                e.stop();
            }
        }
        for a in self.accepted.values() {
            if let Some(e) = &a.edit {
                e.stop();
            }
        }
        self.pending.clear();
        self.states.clear();
        self.accepted.clear();
        self.inflight = None;
        self.notice = "";
    }
    pub fn suspend(&mut self) {
        self.retire_dispatch(None, true);
        for c in &self.pending {
            if let Some(e) = &c.edit {
                e.stop();
            }
        }
        self.pending
            .retain(|c| matches!(c.kind, Kind::Cancel { .. }));
        // Pausing visuals does not close views; retain bounded, expiring ACK identities, and still require explicit review of fresh prompts after resuming.
        // clear or observe retires actual layout/lease/generation changes; pause is not implicit rejection.
    }
    pub fn offer(&mut self, commands: Vec<Command>, now: Instant) -> bool {
        self.prune(now);
        if commands.is_empty() {
            return false;
        }
        let replace = crate::tile_surface_edits::replace_tail(&self.pending, &commands);
        let (held_slots, held_bytes) = self.held_cost();
        if self.pending.len() + commands.len() + held_slots - usize::from(replace) > 32 {
            self.notice = "tile-art-action-queue-full";
            return false;
        }
        // Charge conservatively per anchor, including each shared Arc, so accumulated old scenes/prompts/JSON stay within 8MiB.
        let bytes = self
            .pending
            .iter()
            .take(self.pending.len() - usize::from(replace))
            .chain(commands.iter())
            .map(Command::bytes)
            .sum::<usize>()
            + held_bytes
            + self
                .accepted
                .values()
                .map(|a| a.anchor.bytes())
                .sum::<usize>();
        if bytes > 8 * 1024 * 1024 {
            self.notice = "tile-art-action-memory";
            return false;
        }
        if commands.iter().any(|c| {
            now >= c.until
                || c.edit.as_ref().is_some_and(|e| !e.active())
                || self
                    .states
                    .get(&c.anchor.view.instance_id)
                    .is_none_or(|s| s.view != c.anchor.view || s.generation != c.anchor.generation)
        }) {
            return false;
        }
        #[cfg(test)]
        {
            self.test_offered += commands.len() as u64;
        }
        if replace {
            self.pending.pop_back();
        }
        let retiring: Vec<_> = commands
            .iter()
            .filter_map(|c| match &c.kind {
                Kind::Cancel { event, .. } => self.accepted.get(event).and_then(|a| a.edit.clone()),
                _ => None,
            })
            .collect();
        for edit in retiring {
            self.cancel_edit(&edit);
        }
        self.pending.extend(commands);
        true
    }
    pub fn observe(&mut self, state: Arc<State>, origin: &str, device: &str, now: Instant) {
        self.prune(now);
        let instance = &state.view.instance_id;
        if self
            .states
            .get(instance)
            .is_some_and(|old| old.view != state.view || old.generation != state.generation)
        {
            self.discard(instance);
        }
        if state.activity.failure.is_some()
            && self
                .states
                .get(instance)
                .is_none_or(|old| old.activity.failure != state.activity.failure)
        {
            self.retire_dispatch(Some(instance), false);
            for c in self
                .pending
                .iter()
                .filter(|c| c.anchor.view.instance_id == *instance)
            {
                if let Some(e) = &c.edit {
                    e.stop();
                }
            }
            for a in self
                .accepted
                .values()
                .filter(|a| a.anchor.view.instance_id == *instance)
            {
                if let Some(e) = &a.edit {
                    e.stop();
                }
            }
            self.pending
                .retain(|c| c.anchor.view.instance_id != *instance);
            self.notice = "tile-art-action-failed";
        }
        self.release_ready_edits(&state, now);
        self.accepted.retain(|event, a| {
            now < a.until
                && (a.anchor.view.instance_id != *instance
                    || a.anchor.view == state.view && a.anchor.generation == state.generation)
                && (a.edit.is_none()
                    || a.anchor.view.instance_id != *instance
                    || state
                        .activity
                        .pending
                        .iter()
                        .any(|p| p.event == *event && p.ack.request_id() == a.request)
                    || state
                        .activity
                        .confirmations
                        .iter()
                        .any(|c| c.event_id == *event && c.request_id == a.request))
        });
        self.origin = origin.into();
        self.device = device.into();
        self.states.insert(instance.clone(), state);
    }
    pub fn take(&mut self, state: &State, now: Instant) -> Option<Command> {
        self.prune(now);
        let index = self.pending.iter().position(|c| {
            c.anchor.view == state.view
                && c.anchor.generation == state.generation
                && (!matches!(c.kind, Kind::Event(_))
                    || state.activity.pending.is_empty() && state.activity.confirmations.is_empty())
        })?;
        let command = self.pending.remove(index)?;
        if let Some(edit) = &command.edit {
            edit.sending(Some(now));
            self.inflight = Some((
                command.anchor.view.clone(),
                command.anchor.generation,
                Arc::downgrade(edit),
            ));
        }
        Some(command)
    }
    pub fn cancel_for(&mut self, view: &View, now: Instant) -> bool {
        self.prune(now);
        self.pending
            .iter()
            .any(|c| c.anchor.view == *view && matches!(c.kind, Kind::Cancel { .. }))
    }
    pub fn take_cancel(&mut self, state: &State, now: Instant) -> Option<Command> {
        self.prune(now);
        let index = self.pending.iter().position(|c| {
            c.anchor.view == state.view
                && c.anchor.generation == state.generation
                && now < c.until
                && matches!(c.kind, Kind::Cancel { .. })
        })?;
        self.pending.remove(index)
    }
    pub fn attempted(&mut self, command: &Command, ack: &Ack, now: Instant) {
        let rejected = !ack.accepted()
            || matches!(
                ack.status(),
                Status::Failed | Status::Cancelled | Status::Interrupted | Status::Unknown
            );
        if rejected {
            if let Some(edit) = &command.edit {
                self.cancel_edit(edit);
            }
        }
        self.notice = if rejected {
            "tile-art-action-rejected"
        } else if ack.status() == Status::AwaitingConfirmation {
            "tile-art-action-confirmation"
        } else {
            "tile-art-action-accepted"
        };
        if let Kind::Event(event) = &command.kind {
            if !rejected && (ack.status() == Status::AwaitingConfirmation || command.edit.is_some())
            {
                self.prune(now);
                let bytes = self.pending.iter().map(Command::bytes).sum::<usize>()
                    + self
                        .accepted
                        .values()
                        .map(|a| a.anchor.bytes())
                        .sum::<usize>()
                    + command.anchor.bytes();
                if self.accepted.len() >= 64 || bytes > 8 * 1024 * 1024 {
                    if let Some(edit) = &command.edit {
                        self.cancel_edit(edit);
                    }
                    self.notice = "tile-art-action-capacity";
                    return;
                }
                let event_id = event["eventId"].as_str().unwrap().to_owned();
                self.accepted.insert(
                    event_id,
                    Accepted {
                        anchor: command.anchor.clone(),
                        request: ack.request_id().into(),
                        action: event["action"].as_str().unwrap().into(),
                        until: now + WAIT_LIMIT,
                        edit: command.edit.clone(),
                    },
                );
            }
        } else if let Kind::Decision {
            confirmation,
            approved,
        } = &command.kind
        {
            if !approved || rejected {
                if let Some(edit) = self
                    .accepted
                    .get(&confirmation.event_id)
                    .and_then(|a| a.edit.clone())
                {
                    self.cancel_edit(&edit);
                }
            }
            // After Slider approval, keep tracking the original request until fresh state proves terminal execution; otherwise cancellation/failure loses dependencies.
            if !approved
                || rejected
                || self
                    .accepted
                    .get(&confirmation.event_id)
                    .is_none_or(|a| a.edit.is_none())
            {
                self.accepted.remove(&confirmation.event_id);
            }
        }
        if let Some(edit) = &command.edit {
            edit.sending(None);
            self.inflight = None;
        }
    }
    pub fn failed(&mut self, instance: &str) {
        self.discard(instance);
        self.notice = "tile-art-action-unknown";
    }
    fn discard(&mut self, instance: &str) {
        self.retire_dispatch(Some(instance), false);
        for c in self
            .pending
            .iter()
            .filter(|c| c.anchor.view.instance_id == instance)
        {
            if let Some(e) = &c.edit {
                e.stop();
            }
        }
        for a in self
            .accepted
            .values()
            .filter(|a| a.anchor.view.instance_id == instance)
        {
            if let Some(e) = &a.edit {
                e.stop();
            }
        }
        self.pending
            .retain(|c| c.anchor.view.instance_id != instance);
        self.accepted
            .retain(|_, a| a.anchor.view.instance_id != instance);
    }
    fn prune(&mut self, now: Instant) {
        let expired: Vec<_> = self
            .pending
            .iter()
            .filter(|c| now >= c.until && !self.waiting_edit(c, now))
            .filter_map(|c| c.edit.clone())
            .collect();
        for edit in expired {
            self.cancel_edit(&edit);
            self.notice = "tile-art-action-expired";
        }
        let ready: Vec<_> = self
            .pending
            .iter()
            .map(|c| now < c.until || self.waiting_edit(c, now))
            .collect();
        let mut ready = ready.into_iter();
        self.pending.retain(|_| {
            let alive = ready.next().unwrap();
            if !alive {
                self.notice = "tile-art-action-expired";
            }
            alive
        });
        self.accepted.retain(|_, a| {
            if now >= a.until {
                if let Some(edit) = &a.edit {
                    edit.stop();
                }
                return false;
            }
            true
        });
    }
    pub fn owns_confirmation(
        &self,
        c: &crate::tile_surface_activity::Confirmation,
        state: &State,
        now_ms: u64,
    ) -> Option<&Accepted> {
        self.accepted.get(&c.event_id).filter(|a| {
            a.anchor.view == state.view
                && a.anchor.generation == state.generation
                && a.request == c.request_id
                && a.action == c.action_id
                && c.device_id == self.device
                && now_ms < c.expires_at_ms
                && Instant::now() < a.until
        })
    }
}
impl Anchor {
    fn bytes(&self) -> usize {
        self.snapshot.scene.len()
            + self
                .snapshot
                .images
                .iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>()
            + self
                .snapshot
                .dimensions
                .keys()
                .map(|k| k.len() + 32)
                .sum::<usize>()
            + self.placement.len()
            + self.scroll.bytes()
            + 1024
    }
}
impl Command {
    fn bytes(&self) -> usize {
        self.anchor.bytes()
            + match &self.kind {
                Kind::Event(v) => serde_json::to_vec(v).map_or(usize::MAX / 128, |v| v.len()),
                Kind::Decision {
                    confirmation: c, ..
                } => c.payload.as_ref().map_or(0, |v| v.to_string().len()) + 2048,
                Kind::Cancel { request, event } => request.len() + event.len(),
            }
    }
}
#[path = "tile_surface_action_pending.rs"]
pub(crate) mod pending;
#[cfg(test)]
#[path = "tile_surface_action_tests.rs"]
pub(crate) mod tests;
