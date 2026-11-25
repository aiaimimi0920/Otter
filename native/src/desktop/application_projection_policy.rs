//! The original profile owner persists consent; only successful commit elevates acceptance permission, while failure preserves committed policy.
use super::super::*;
use crate::{
    desktop::{
        device_authorization_ui::{policy::Consent, Request},
        loom_execution_signal::Notify,
    },
    projection_receive_policy::{Change, Policy},
};
use std::{path::Path, sync::atomic::AtomicBool};
#[path = "application_projection_policy_file.rs"]
mod file;
#[path = "application_projection_policy_job.rs"]
mod job;
#[path = "application_projection_policy_schedule.rs"]
mod schedule;
pub(super) use schedule::Schedule;

#[derive(Default)]
pub(in crate::desktop::application) struct Owner {
    file: Option<file::File>,
    initial: bool,
    job: Option<job::Job>,
    pub(super) schedule: Schedule,
}

impl Owner {
    pub(super) fn load(root: &Path, cancel: &AtomicBool) -> Self {
        Self {
            file: Some(file::File::load(root, cancel)),
            initial: true,
            ..Default::default()
        }
    }
    pub(super) fn reap_deadline(&self) -> Option<std::time::Instant> {
        self.job.as_ref().and_then(job::Job::reap_deadline)
    }
    fn discard(&mut self, consent: &mut Consent) {
        // Closed views cannot publish staged data; retain the original worker until exit without synchronous message-pump waiting.
        if let Some(job) = self.job.as_ref() {
            job.stop();
        }
        self.job.take_if(|job| job.finished());
        if self.job.is_none() {
            consent.finish_save();
        }
    }
    pub(super) fn poll(&mut self, consent: &mut Consent) {
        if self.initial {
            self.initial = false;
            let file = self.file.as_ref().expect("loaded policy");
            consent.install(file.policy());
            consent.install_paused(file.paused());
            consent.status = if file.writable() {
                ""
            } else {
                crate::ui_text::text("app-projection-policy-unreadable")
            };
        }
        let Some(result) = self.job.as_mut().and_then(job::Job::poll) else {
            return;
        };
        let stopped = self.job.take().unwrap().stopped();
        consent.finish_save();
        if stopped {
            return; // Reopening the view must not permit committing a canceled result.
        }
        match result.and_then(file::Prepared::commit) {
            Ok((next, retained)) => {
                consent.install(next.policy());
                consent.install_paused(next.paused());
                self.file = Some(next);
                self.schedule.reset();
                consent.status = if retained {
                    crate::ui_text::text("app-projection-policy-recovery-retained")
                } else {
                    crate::ui_text::text("app-projection-policy-saved")
                };
            }
            Err(_) => consent.status = crate::ui_text::text("app-projection-policy-save-failed"),
        }
    }
    fn request(&mut self, change: impl Into<Change>, consent: &mut Consent, notify: Notify) {
        if self.job.is_some() || consent.saving {
            return;
        }
        let change = change.into();
        if match change {
            Change::Policy(policy) => consent.policy() == policy,
            Change::Selection(policy) => consent.policy() == policy && !consent.paused(),
            Change::Paused(paused) => consent.paused() == paused,
        } {
            return;
        }
        let Some(file) = self.file.as_ref().filter(|file| file.writable()) else {
            consent.status = crate::ui_text::text("app-projection-policy-save-failed");
            return;
        };
        match job::Job::start(file.clone(), change, notify) {
            Ok(job) => {
                self.job = Some(job);
                match change {
                    Change::Policy(_) => consent.begin_save(),
                    Change::Selection(_) => consent.begin_pause(),
                    Change::Paused(_) => consent.begin_pause(),
                }
                self.schedule.reset();
            }
            Err(_) => consent.status = crate::ui_text::text("app-projection-policy-save-failed"),
        }
    }
}
impl Application {
    pub(in crate::desktop::application) fn process_projection_policy(&mut self) {
        let mut view = self.authorization.borrow_mut();
        let owner = match &mut self.projection_profile {
            super::profile::Owner::Ready { policy, .. } => Some(policy),
            _ => None,
        };
        let mut owner = owner;
        if view.closed || self.exit_requested.get() {
            if let Some(owner) = owner {
                owner.discard(&mut view.consent);
            } else {
                view.consent.finish_save();
            }
            return;
        }
        if let Some(owner) = owner.as_mut() {
            owner.poll(&mut view.consent);
        }
        let change = match &view.request {
            Some(Request::ReceivePolicy(policy)) => Some(Change::Selection(*policy)),
            Some(Request::ReceivePause(paused)) => Some(Change::Paused(*paused)),
            _ => None,
        };
        if let Some(change) = change {
            view.request.take();
            if view.closed || self.exit_requested.get() || self.modals.blocked() {
                return;
            }
            if let Some(owner) = owner {
                owner.request(change, &mut view.consent, self.execution_wake.callback());
                if matches!(change, Change::Paused(_) | Change::Selection(_)) && view.consent.saving
                {
                    // Immediately clear old menus/credentials; the original slot still reaps networking, and neither failure nor recovery revives old snapshots.
                    view.projection.entries.clear();
                    view.projection.participation = None;
                }
            } else {
                view.consent.status = crate::ui_text::text("app-projection-policy-save-failed");
            }
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "application_projection_policy_tests.rs"]
mod tests;
