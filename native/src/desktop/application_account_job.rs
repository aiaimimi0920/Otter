//! Single-slot account tasks: borrow socket cancellation and completion signals from the original Device Lease; start no subsequent chain before reaping.
use super::*;
use crate::{device_authorization_lease::Lease, projection_account as account};
use std::{
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
};

pub(super) struct Job {
    lease: Arc<Lease>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Output>>>,
}
impl Job {
    pub(super) fn start(
        lease: Lease,
        notify: crate::desktop::loom_execution_signal::Notify,
        work: impl FnOnce(&Lease) -> Result<Output> + Send + 'static,
    ) -> Result<Self> {
        let lease = Arc::new(lease);
        lease.notify_revocation(notify.clone());
        let worker_lease = lease.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-account-projection".into())
            .spawn(move || {
                let _completion =
                    crate::desktop::loom_execution_signal::CompletionSignal::new(completed, notify);
                let lease = worker_lease;
                let _com = crate::desktop::Apartment::new()?;
                work(&lease)
            })
            .map_err(|e| Error::new(E_FAIL, format!("Account projection worker: {e}")))?;
        Ok(Self {
            lease,
            ready,
            worker: Some(worker),
        })
    }
    pub(super) fn valid(&self, origin: &str) -> bool {
        crate::desktop::surface_windows::now_ms()
            .is_ok_and(|now| self.lease.validate(origin, now).is_ok())
    }
    pub(super) fn stop(&self) {
        self.lease.revoke();
    }
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.worker.as_ref()?;
        crate::desktop::loom_execution_signal::reap_deadline(&self.ready)
    }
    pub(super) fn poll(&mut self) -> Option<Result<Output>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .unwrap_or_else(|_| Err(Error::new(E_FAIL, "Account projection worker panicked"))),
        )
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            self.stop();
            let _ = worker.join();
        }
    }
}
pub(super) fn failure(error: account::Error) -> Error {
    if let account::Error::Remote {
        code: Some(code), ..
    } = &error
    {
        if matches!(
            code.as_str(),
            "account_signed_out" | "projection_account_required"
        ) {
            return Error::new(
                E_FAIL,
                crate::ui_text::text("app-account-share-not-signed-in"),
            );
        }
        if matches!(
            code.as_str(),
            "projection_unavailable" | "projection_not_configured" | "account_not_configured"
        ) {
            return Error::new(
                E_FAIL,
                crate::ui_text::text("app-account-share-unconfigured"),
            );
        }
    }
    Error::new(E_FAIL, format!("Account projection: {error:?}"))
}
pub(super) fn content(state: account::State) -> Result<Box<account::Content>> {
    match state {
        account::State::Content(content) => Ok(content),
        _ => Err(stale()),
    }
}
