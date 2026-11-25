//! Bounded fair scheduling: separate NEW from accepted entries missing records, both reusing the Sessions slot and original receiver owner.
use super::*;
use crate::{
    desktop::device_authorization_ui::projection::Request, projection_delivery::Invitation,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, PartialEq, Eq)]
struct Key(String, u64, String, bool, String, Option<String>);
impl Key {
    fn matches(&self, entry: &Invitation) -> bool {
        self.0 == entry.envelope().projection_id()
            && self.1 == entry.revision()
            && self.2 == entry.digest()
            && self.3 == entry.accepted()
            && self.4 == entry.origin()
            && self.5.as_deref() == entry.peer_id()
    }
}
fn key(entry: &Invitation) -> Key {
    Key(
        entry.envelope().projection_id().to_owned(),
        entry.revision(),
        entry.digest().to_owned(),
        entry.accepted(),
        entry.origin().into(),
        entry.peer_id().map(str::to_owned),
    )
}
fn missing(
    entry: &Invitation,
    records: &[Arc<crate::projection_delivery::journal::Ticket>],
    units: &[crate::workspace_id::UnitId],
) -> bool {
    let unit_id = entry.receiver_unit_id();
    !units.iter().any(|unit| unit.as_str() == unit_id)
        && !records.iter().any(|ticket| {
            ticket.origin() == entry.origin()
                && ticket.invitation().peer_id() == entry.peer_id()
                && ticket.envelope().projection_id() == entry.envelope().projection_id()
        })
}
enum Attempt {
    FirstAccept,
    AwaitingInbox,
    RetryReady,
    Done,
}

#[derive(Default)]
pub(in crate::desktop::application) struct Schedule {
    origin: Option<String>,
    attempted: Vec<(Key, Attempt)>,
    last: Option<String>,
    next: Option<Instant>,
    pub(super) wake: Option<Instant>,
    failures: u32,
}
impl Schedule {
    #[cfg(test)]
    pub(in crate::desktop::application) fn defer_for_test(&mut self, origin: &str) {
        // Manual protocol fixtures exclusively own their server script; move only the actual deadline without changing production scheduler branches.
        self.origin = Some(origin.to_owned());
        self.next = Some(Instant::now() + Duration::from_secs(3600));
        self.wake = self.next;
    }
    pub(super) fn reset(&mut self) {
        *self = Self::default();
    }
    pub(in crate::desktop::application) fn inbox(&mut self, entries: &[Arc<Invitation>]) {
        self.attempted
            .retain(|(old, _)| entries.iter().any(|entry| old.matches(entry)));
        for (_, attempt) in &mut self.attempted {
            if matches!(attempt, Attempt::AwaitingInbox) {
                *attempt = Attempt::RetryReady;
            }
        }
        self.failures = 0;
    }
    pub(in crate::desktop::application) fn accept_unapplied(&mut self, entry: &Invitation) -> bool {
        if let Some((_, attempt @ Attempt::FirstAccept)) = self
            .attempted
            .iter_mut()
            .find(|(old, _)| old.matches(entry))
        {
            *attempt = Attempt::AwaitingInbox;
            true
        } else {
            false
        }
    }
    fn align_origin(&mut self, origin: &str) {
        if self.origin.as_deref() != Some(origin) {
            self.reset();
            self.origin = Some(origin.to_owned());
        }
    }
    pub(in crate::desktop::application) fn failure(&mut self, origin: &str, now: Instant) {
        self.align_origin(origin);
        self.failures = (self.failures + 1).min(4);
        self.next = now.checked_add(Duration::from_millis((3000 << self.failures).min(30_000)));
        self.wake = self.next;
    }
    pub(in crate::desktop::application) fn completed(&mut self, origin: &str, now: Instant) {
        self.align_origin(origin);
        // Measure intervals from actual completion; a slow inbox cannot immediately retake the slot and must leave room for existing associations' 750ms sync.
        self.failures = 0;
        self.next = now.checked_add(Duration::from_secs(3));
        self.wake = self.next;
    }
    fn due(&mut self, origin: &str, now: Instant) -> bool {
        self.align_origin(origin);
        if let Some(next) = self.next.filter(|next| *next > now) {
            self.wake = Some(next);
            return false;
        }
        true
    }
    fn plan(
        &mut self,
        origin: &str,
        entries: &[Arc<Invitation>],
        policy: Policy,
        now: Instant,
    ) -> Option<Request> {
        if !self.due(origin, now) {
            return None;
        }
        self.next = now.checked_add(Duration::from_secs(3));
        self.wake = self.next; // Pre-start failures still retain bounded wakeups, without relying on notifications from a nonexistent worker.
                               // Retry only after the authoritative inbox confirms the same invitation still exists; never reuse the pre-failure snapshot.
        if self
            .attempted
            .iter()
            .any(|(_, attempt)| matches!(attempt, Attempt::AwaitingInbox))
        {
            return Some(Request::Refresh);
        }
        let count = entries
            .len()
            .min(crate::projection_delivery::MAX_INVITATIONS);
        let start = entries
            .iter()
            .position(|entry| Some(entry.envelope().projection_id()) == self.last.as_deref())
            .map_or(0, |index| index + 1);
        for offset in 0..count {
            let entry = &entries[(start + offset) % count];
            if entry.origin() != origin {
                continue;
            }
            if let Some((_, attempt)) = self
                .attempted
                .iter_mut()
                .find(|(old, _)| old.matches(entry))
            {
                if !matches!(attempt, Attempt::RetryReady) {
                    continue;
                }
                *attempt = Attempt::Done;
            } else {
                // Snapshots contain at most 64 entries; when full, query only, without evicting old credentials to retry the same failed invitation repeatedly.
                if self.attempted.len() >= crate::projection_delivery::MAX_INVITATIONS {
                    continue;
                }
                let attempt = if !entry.accepted() && policy != Policy::Reject {
                    Attempt::FirstAccept
                } else {
                    Attempt::Done
                };
                self.attempted.push((key(entry), attempt));
            }
            self.last = Some(entry.envelope().projection_id().to_owned());
            return Some(if entry.accepted() {
                Request::Reattach(entry.clone())
            } else if policy == Policy::Reject {
                Request::Reject(entry.clone())
            } else {
                Request::Accept(entry.clone())
            });
        }
        Some(Request::Refresh)
    }
}
impl Application {
    #[cfg(test)]
    pub(in crate::desktop::application) fn resume_projection_poll_for_test(&mut self) {
        if let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
            policy.schedule.reset();
        }
    }
    #[cfg(test)]
    pub(in crate::desktop::application) fn defer_projection_poll_for_test(&mut self, origin: &str) {
        if let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
            policy.schedule.defer_for_test(origin);
        }
    }
    pub(in crate::desktop::application) fn projection_policy_deadline(&self) -> Option<Instant> {
        let profile = &self.projection_profile;
        let inbox = match profile {
            super::super::profile::Owner::Ready { policy, .. } => policy.schedule.wake,
            _ => None,
        };
        inbox.into_iter().chain(profile.reap_deadline()).min()
    }
    pub(in crate::desktop::application) unsafe fn process_projection_automatic(
        &mut self,
        config: &mut impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
        now: Instant,
    ) {
        let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile
        else {
            return;
        };
        policy.schedule.wake = None;
        let view = self.authorization.borrow();
        let mode = view.consent.policy();
        let origin = view.origin.clone();
        let idle = !view.closed
            && !view.busy
            && !view.account.busy
            && !view.projection.busy
            && view.request.is_none()
            && !view.consent.saving
            && view.consent.participation().is_some()
            && !self.exit_requested.get()
            && !self.modals.blocked()
            && !self.projection_receive.busy()
            && !self.projection_edit.busy()
            && !self.remote_sessions.projection_pending()
            && self.save_job.is_none()
            && self.restore_job.is_none()
            && !self.checkpoint_is_exiting();
        if !idle {
            return;
        }
        let Some(origin) = origin else { return };
        if !crate::desktop::surface_windows::now_ms()
            .is_ok_and(|now| self.authorization_owner.token(&origin, now).is_some())
        {
            return;
        }
        if let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile {
            // Active windows may wake the application frequently; do not prepare Scope/candidates or allocate UnitId before the deadline.
            if !policy.schedule.due(&origin, now) {
                return;
            }
        }
        // Check the destination before occupying a candidate; temporary inability to append must not consume the invitation's automatic attempt.
        drop(view);
        let existing = super::super::receive::Scope::capture_existing(self).ok();
        // Receive already expresses device-level consent; like Automatic, acceptance follows existing validation/save paths without a second approval.
        let scope = if mode != Policy::Reject {
            super::super::receive::Scope::capture(self).ok()
        } else {
            None
        };
        let mut view = self.authorization.borrow_mut();
        // Journaled associations may belong to other saved workspaces; do not migrate, overwrite, or queue compensation in the background.
        let entries: Vec<_> = view
            .projection
            .entries
            .iter()
            .filter(|entry| {
                (!entry.accepted() && (mode == Policy::Reject || scope.is_some()))
                    || (entry.accepted()
                        && existing.is_some()
                        && missing(
                            entry,
                            self.projection_profile.entries().unwrap_or(&[]),
                            self.workspace.units(),
                        ))
            })
            .cloned()
            .collect();
        let super::super::profile::Owner::Ready { policy, .. } = &mut self.projection_profile
        else {
            return;
        };
        let Some(request) = policy.schedule.plan(&origin, &entries, mode, now) else {
            return;
        };
        let scope = if matches!(request, Request::Reattach(_)) {
            existing
        } else {
            scope
        };
        if super::super::start(
            &mut view,
            request,
            &mut self.authorization_owner,
            &mut self.remote_sessions,
            &self.projection_profile,
            config,
        ) {
            self.projection_receive.waiting(scope);
        } else if let super::super::profile::Owner::Ready { policy, .. } =
            &mut self.projection_profile
        {
            policy.schedule.failure(&origin, now);
        }
    }
}

#[cfg(test)]
#[path = "application_projection_policy_schedule_tests.rs"]
mod tests;
