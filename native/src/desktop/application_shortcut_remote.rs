//! One authenticated outbound exchange per active Device, with a separate profile generation.
use super::{shortcut_save::Save, Application};
use crate::{
    loom_locale_protocol::{Receipt, POLL_INTERVAL},
    loom_locale_runtime::{Error, Job},
    loom_shortcut_protocol::{Report, Snapshot, PROTOCOL},
};
use ring::rand::{SecureRandom, SystemRandom};
use std::time::{Duration, Instant};
type Target = (String, String);
#[derive(Default)]
pub(super) struct Owner {
    pub enabled: bool,
    target: Option<Target>,
    receiver: String,
    generation: Option<String>,
    next: Option<Instant>,
    job: Option<Job<Snapshot>>,
    pending: Option<Save>,
    receipt: Option<Receipt>,
}
impl Owner {
    #[cfg(test)]
    pub(super) fn test_pending(&mut self, save: Save) {
        self.enabled = true;
        self.pending = Some(save);
    }
    pub(super) fn saving(&self) -> bool {
        self.pending.is_some()
    }
    pub(super) fn reset(&mut self) {
        if let Some(job) = &self.job {
            job.stop();
        }
        if let Some(save) = &self.pending {
            save.stop();
        }
        self.target = None;
        self.receiver.clear();
        self.generation = None;
        self.next = None;
        self.receipt = None;
    }
    pub(super) fn shutdown(&mut self) {
        self.enabled = false;
        self.reset();
        self.job.take();
        self.pending.take();
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.job
            .as_ref()
            .and_then(Job::deadline)
            .into_iter()
            .chain(self.pending.as_ref().and_then(Save::deadline))
            .chain(self.next.filter(|_| self.job.is_none() && !self.saving()))
            .min()
    }
    fn select(&mut self, target: Option<Target>) {
        if self.target == target {
            return;
        }
        self.reset();
        if target.is_some() {
            let mut bytes = [0u8; 16];
            if SystemRandom::new().fill(&mut bytes).is_err() {
                return;
            }
            self.receiver = bytes.iter().map(|b| format!("{b:02x}")).collect();
            self.next = Some(Instant::now());
        }
        self.target = target;
    }
}
impl Application {
    pub(super) unsafe fn process_remote_shortcuts(&mut self) {
        if !self.shortcut_remote.enabled {
            return;
        }
        let now = crate::desktop::surface_windows::now_ms().unwrap_or(u64::MAX);
        let target = if self.exit_requested.get()
            || self.exit_after_save
            || self.exit_committed
            || self.checkpoint_exit_waiting()
            || self.session_end.paused()
        {
            None
        } else {
            let view = self.authorization.borrow();
            view.origin
                .as_ref()
                .zip(view.loom_id.as_ref())
                .filter(|(origin, _)| {
                    !view.closed
                        && view.connected
                        && self.authorization_owner.token(origin, now).is_some()
                })
                .map(|(origin, loom)| (origin.clone(), loom.clone()))
        };
        let mut owner = std::mem::take(&mut self.shortcut_remote);
        owner.select(target.clone());
        if let Some(result) = owner.pending.as_mut().and_then(|s| s.poll(self)) {
            let save = owner.pending.take().unwrap();
            // Exit was checked before task settlement; removing the final save also
            // removes its deadline, so a fresh wake must recheck the waiting intent.
            self.execution_wake.callback()();
            if target
                .as_ref()
                .zip(owner.generation.as_ref())
                .is_some_and(|((origin, loom), g)| save.source == format!("{loom}|{origin}|{g}"))
            {
                owner.receipt = Some(result);
                owner.next = Some(Instant::now());
            }
        }
        if let Some(result) = owner.job.as_mut().and_then(Job::poll) {
            let job = owner.job.take().unwrap();
            if let Some((origin, loom)) = &target {
                if !job.cancel.load(std::sync::atomic::Ordering::Acquire)
                    && job.lease.validate(origin, now).is_ok()
                {
                    match result {
                        Ok(snapshot) => {
                            if job.sent_receipt == owner.receipt {
                                owner.receipt = None;
                            }
                            owner.generation = Some(snapshot.receiver_generation.clone());
                            if owner.receipt.is_none() && owner.pending.is_none() {
                                if let Some(r) = snapshot.request {
                                    let until = (job.started
                                        + Duration::from_millis(r.valid_for_ms))
                                    .min(job.authorized_until)
                                    .min(
                                        Instant::now()
                                            + Duration::from_millis(
                                                r.expires_at.saturating_sub(now).min(15_000),
                                            ),
                                    );
                                    let source =
                                        format!("{loom}|{origin}|{}", snapshot.receiver_generation);
                                    match self.start_shortcut_save(
                                        source,
                                        r,
                                        until,
                                        job.cancel.clone(),
                                        job.lease.clone(),
                                    ) {
                                        Ok(save) => owner.pending = Some(save),
                                        Err(result) => owner.receipt = Some(result),
                                    }
                                }
                            }
                            owner.next = Some(
                                Instant::now()
                                    + if owner.receipt.is_some() {
                                        Duration::ZERO
                                    } else {
                                        POLL_INTERVAL
                                    },
                            );
                        }
                        Err(error) => {
                            if matches!(
                                error,
                                Error::Http(crate::live_relay_http::Error::Status(409))
                            ) {
                                owner.generation = None;
                                owner.receipt = None;
                            }
                            owner.next = Some(Instant::now() + Duration::from_secs(15));
                        }
                    }
                }
            }
        }
        if let Some((origin, loom)) = target.filter(|_| {
            owner.job.is_none()
                && owner.pending.is_none()
                && owner.next.is_some_and(|d| d <= Instant::now())
        }) {
            owner.next = Some(Instant::now() + POLL_INTERVAL);
            let host = self
                .documents
                .iter()
                .find(|d| d.is_alive() && crate::desktop::shortcut_registry::owns(&d.state))
                .map(|doc| &doc.state);
            let settings = self
                .naming_document
                .shortcut_settings()
                .expect("validated shortcut settings");
            let report = Report {
                protocol: PROTOCOL,
                receiver_id: owner.receiver.clone(),
                receiver_generation: owner.generation.clone(),
                revision: settings.revision,
                policy: settings.policy,
                enabled: self.hotkey_enabled.get(),
                suppressed: host.is_some_and(|s| s.shortcut_host.suppressed.get()),
                registered: host
                    .map(|s| crate::desktop::shortcut_registry::registered(s))
                    .unwrap_or_default(),
                result: owner.receipt.clone(),
            };
            if let Some((config, lease)) =
                crate::device_authorization_config::Config::from_env(&origin)
                    .ok()
                    .zip(self.authorization_owner.lease(&origin, now).ok())
            {
                owner.job = Job::exchange(
                    config,
                    lease,
                    owner.receipt.clone(),
                    self.execution_wake.callback(),
                    "otter-shortcut-report",
                    move |control| control.report_shortcuts(&loom, &report),
                )
                .ok();
            }
        }
        self.shortcut_remote = owner;
    }
}
