//! One cancellable locale exchange; leases remain alive through UI settlement and never expose tokens.
use crate::{
    desktop::loom_execution_signal::{reap_deadline, CompletionSignal, Notify},
    device_authorization_config::Config,
    device_authorization_lease::Lease,
    loom_locale_protocol::{Receipt, Report, Snapshot},
};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub struct Input {
    pub loom_id: String,
    pub receiver_id: String,
    pub generation: Option<String>,
    pub language: crate::display_language::DisplayLanguage,
    pub revision: u64,
    pub receipt: Option<Receipt>,
}

#[derive(Debug)]
pub enum Error {
    Preparation,
    Revoked,
    Http(crate::live_relay_http::Error),
    Worker,
}

pub struct Job {
    pub lease: Arc<Lease>,
    pub cancel: Arc<AtomicBool>,
    pub started: Instant,
    pub authorized_until: Instant,
    pub sent_receipt: Option<Receipt>,
    ready: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<Snapshot, Error>>>,
}

impl Job {
    pub fn start(config: Config, lease: Lease, input: Input, notify: Notify) -> io::Result<Self> {
        let lease = Arc::new(lease);
        let worker_lease = lease.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let revoked = cancel.clone();
        let revoke_notify = notify.clone();
        lease.notify_revocation(Arc::new(move || {
            revoked.store(true, Ordering::Release);
            revoke_notify();
        }));
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let started = Instant::now();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?;
        let remaining = lease
            .expires_at_ms()
            .saturating_sub(u64::try_from(now.as_millis()).map_err(io::Error::other)?);
        let authorized_until = started
            .checked_add(Duration::from_millis(remaining.min(15_000)))
            .ok_or_else(|| io::Error::other("Locale authorization deadline overflow"))?;
        let sent_receipt = input.receipt.clone();
        let thread = std::thread::Builder::new()
            .name("otter-locale-report".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                let client = worker_lease
                    .prepare_client(&config)
                    .map_err(|_| Error::Preparation)?;
                let report = Report {
                    receiver_id: &input.receiver_id,
                    receiver_generation: input.generation.as_deref(),
                    language: input.language.code(),
                    revision: input.revision,
                    supported_languages: ["en", "zh-Hans"],
                    result: input.receipt.as_ref(),
                };
                worker_lease
                    .control(&client)
                    .map_err(|_| Error::Revoked)?
                    .report_locale(&input.loom_id, &report)
                    .map_err(Error::Http)
            })?;
        Ok(Self {
            lease,
            cancel,
            started,
            authorized_until,
            sent_receipt,
            ready,
            thread: Some(thread),
        })
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.thread.as_ref()?;
        reap_deadline(&self.ready)
    }

    pub fn poll(&mut self) -> Option<Result<Snapshot, Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let result = self.thread.take()?.join().unwrap_or(Err(Error::Worker));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err(Error::Revoked)
        } else {
            result
        })
    }

    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
        self.lease.revoke();
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        // A settled job may have transferred its lease to an awaiting UI request.
        // Only an unfinished worker is cancelled and joined by this safeguard.
        if let Some(thread) = self.thread.take() {
            self.stop();
            let _ = thread.join();
        }
    }
}
