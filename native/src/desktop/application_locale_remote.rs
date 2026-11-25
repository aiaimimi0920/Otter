//! Device-scoped locale receiver; requests are one-shot intents, never a persistent Loom override.
use super::{locale, Application};
use crate::{
    device_authorization_lease::Lease,
    loom_locale_protocol::{Receipt, POLL_INTERVAL},
    loom_locale_runtime::{Error, Input, Job},
};
use ring::rand::{SecureRandom, SystemRandom};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, PartialEq, Eq)]
struct Target {
    origin: String,
    loom: String,
}

struct Applying {
    request_id: String,
    source: String,
    cancel: Arc<AtomicBool>,
    lease: Arc<Lease>,
}

#[derive(Default)]
pub(super) struct Owner {
    enabled: bool,
    target: Option<Target>,
    receiver: Option<String>,
    generation: Option<String>,
    next: Option<Instant>,
    job: Option<Job>,
    applying: Option<Applying>,
    receipt: Option<Receipt>,
}

impl Owner {
    pub(super) fn enable(&mut self) {
        self.enabled = true;
    }

    /// An explicit settings-source import starts a new receiver scope. Old CAS
    /// requests must not cross into a profile whose revision may be lower.
    pub(super) fn reset_profile(&mut self) {
        self.cancel();
        self.target = None;
        self.receiver = None;
    }

    pub(super) fn deadline(&self) -> Option<Instant> {
        self.job
            .as_ref()
            .and_then(Job::deadline)
            .into_iter()
            .chain(self.next.filter(|_| self.job.is_none()))
            .min()
    }

    fn select(&mut self, target: Option<Target>) {
        if self.target == target {
            return;
        }
        self.cancel();
        self.target = target;
        // A -> B -> A is a new receiver scope, not permission to resurrect A's pending intent.
        self.receiver = None;
        self.next = self.target.as_ref().map(|_| Instant::now());
    }

    fn cancel(&mut self) {
        if let Some(job) = &self.job {
            job.stop();
        }
        if let Some(applying) = self.applying.take() {
            applying.cancel.store(true, Ordering::Release);
            applying.lease.revoke();
        }
        self.generation = None;
        self.receipt = None;
        self.next = None;
    }

    pub(super) fn shutdown(&mut self) {
        self.enabled = false;
        self.cancel();
        self.job.take();
        self.target = None;
    }

    fn receiver(&mut self) -> Option<&str> {
        if self.receiver.is_none() {
            let mut bytes = [0u8; 16];
            SystemRandom::new().fill(&mut bytes).ok()?;
            self.receiver = Some(bytes.iter().map(|b| format!("{b:02x}")).collect());
        }
        self.receiver.as_deref()
    }

    fn settle_receipt(&mut self, receipt: locale::Receipt) {
        let Some(applying) = self.applying.as_ref() else {
            return;
        };
        if applying.request_id != receipt.request_id || applying.source != receipt.source {
            return;
        }
        let (status, code, applied_revision) = match receipt.outcome {
            locale::Outcome::Applied => ("applied", "applied", Some(receipt.revision)),
            locale::Outcome::Conflict => ("conflict", "revision_conflict", None),
            locale::Outcome::Expired => ("rejected", "request_expired", None),
            locale::Outcome::Failed(_) => ("rejected", "settings_save_failed", None),
        };
        self.receipt = Some(Receipt {
            request_id: receipt.request_id,
            status,
            code,
            applied_revision,
        });
        self.applying.take();
        self.next = Some(Instant::now());
    }

    fn acknowledge(&mut self, sent: Option<&Receipt>) {
        // A report may finish after a newer local result. Retire only its exact receipt.
        if sent.is_some() && self.receipt.as_ref() == sent {
            self.receipt = None;
        }
    }

    fn can_accept_request(&mut self) -> bool {
        // A later response must not replace a result that Loom has not acknowledged.
        if self.receipt.is_some() {
            self.next = Some(Instant::now());
        }
        self.applying.is_none() && self.receipt.is_none()
    }
}

impl Application {
    pub(super) fn process_remote_locale(&mut self) {
        if !self.locale_remote.enabled {
            return;
        }
        let mut owner = std::mem::take(&mut self.locale_remote);
        let now = super::super::surface_windows::now_ms().unwrap_or(u64::MAX);
        let target = if self.exit_requested.get()
            || self.exit_after_save
            || self.exit_committed
            || self.session_end.paused()
        {
            None
        } else {
            let view = self.authorization.borrow();
            view.origin
                .as_ref()
                .zip(view.loom_id.as_ref())
                .and_then(|(origin, loom)| {
                    (!view.closed
                        && view.connected
                        && self.authorization_owner.token(origin, now).is_some())
                    .then(|| Target {
                        origin: origin.clone(),
                        loom: loom.clone(),
                    })
                })
        };
        owner.select(target);
        if let Some(receipt) = self.take_display_language_result() {
            owner.settle_receipt(receipt);
        }
        if let Some(result) = owner.job.as_mut().and_then(Job::poll) {
            let job = owner.job.take().expect("completed locale exchange");
            if !job.cancel.load(Ordering::Acquire) {
                if let Some(target) = owner.target.clone() {
                    if job.lease.validate(&target.origin, now).is_ok() {
                        self.settle_locale_exchange(&mut owner, &target, &job, result);
                    }
                }
            }
        }
        if owner.job.is_none() && owner.next.is_some_and(|at| at <= Instant::now()) {
            owner.next = Some(Instant::now() + POLL_INTERVAL);
            if let Some(target) = owner.target.clone() {
                if let Some(receiver) = owner.receiver().map(str::to_owned) {
                    let status = self.display_language_status();
                    let input = Input {
                        loom_id: target.loom,
                        receiver_id: receiver,
                        generation: owner.generation.clone(),
                        language: status.language,
                        revision: status.revision,
                        receipt: owner.receipt.clone(),
                    };
                    let prepared =
                        crate::device_authorization_config::Config::from_env(&target.origin)
                            .ok()
                            .zip(self.authorization_owner.lease(&target.origin, now).ok());
                    if let Some((config, lease)) = prepared {
                        match Job::start(config, lease, input, self.execution_wake.callback()) {
                            Ok(job) => owner.job = Some(job),
                            Err(_) => owner.next = Some(Instant::now() + Duration::from_secs(15)),
                        }
                    }
                }
            }
        }
        self.locale_remote = owner;
    }

    fn settle_locale_exchange(
        &mut self,
        owner: &mut Owner,
        target: &Target,
        job: &Job,
        result: Result<crate::loom_locale_protocol::Snapshot, Error>,
    ) {
        owner.next = Some(Instant::now() + POLL_INTERVAL);
        let snapshot = match result {
            Ok(snapshot) => snapshot,
            Err(error) => {
                if matches!(
                    error,
                    Error::Http(crate::live_relay_http::Error::Status(409))
                ) {
                    // A restarted daemon or a competing receiver requires a new claim.
                    // Never migrate an uncommitted request into the new generation.
                    owner.cancel();
                }
                owner.next = Some(Instant::now() + Duration::from_secs(15));
                return;
            }
        };
        owner.generation = Some(snapshot.receiver_generation.clone());
        owner.acknowledge(job.sent_receipt.as_ref());
        let Some(request) = snapshot.request else {
            return;
        };
        if !owner.can_accept_request() {
            return;
        }
        let source = format!(
            "{}|{}|{}",
            target.loom, target.origin, snapshot.receiver_generation
        );
        let intent = locale::RemoteRequest {
            request_id: request.request_id.clone(),
            source: source.clone(),
            language: request.language().expect("validated locale"),
            expected_revision: request.expected_revision,
            expires_at: request
                .deadline(job.started)
                .expect("bounded deadline")
                .min(job.authorized_until),
            cancel: job.cancel.clone(),
        };
        match self.request_display_language(intent) {
            Ok(()) => {
                owner.applying = Some(Applying {
                    request_id: request.request_id,
                    source,
                    cancel: job.cancel.clone(),
                    lease: job.lease.clone(),
                });
            }
            Err(_) => {
                owner.receipt = Some(Receipt {
                    request_id: request.request_id,
                    status: "rejected",
                    code: "settings_busy",
                    applied_revision: None,
                });
                owner.next = Some(Instant::now());
            }
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "application_locale_remote_tests.rs"]
mod tests;
