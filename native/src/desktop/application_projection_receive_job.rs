//! Background preparation of one receiver candidate; reuse original PNG/session append/pixel budgets and Wake completion, still awaiting reaping after cancellation.
use super::*;
use crate::desktop::{
    loom_execution_signal::{reap_deadline, CompletionSignal, Notify},
    session_candidate::PreparedRestore,
};
use crate::{
    device_authorization_lease::Lease,
    projection_delivery_runtime::{Hold, Ready},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub(super) struct Prepared {
    pub restore: PreparedRestore,
    pub retained: crate::pixels::Pixels,
    pub held: Hold,
    pub expected: crate::projection_delivery::binding::Binding,
}
struct Candidate {
    restore: PreparedRestore,
    retained: crate::pixels::Pixels,
    expected: crate::projection_delivery::binding::Binding,
}
pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    lease: Arc<Lease>,
    held: Option<Hold>,
    thread: Option<JoinHandle<Result<Candidate>>>,
}
impl Job {
    pub(super) fn start(
        ready: Ready,
        current: &Lease,
        scope: &Scope,
        units: Vec<crate::workspace_id::UnitId>,
        notify: Notify,
    ) -> Result<Self> {
        let (value, held) = ready.transfer(current, now()?).map_err(|_| stale())?;
        let lease = held.lease().clone();
        lease.notify_revocation(notify.clone());
        // The worker only borrows a permission-free snapshot of the original record; failure or panic must not release Slot/activity permission before UI reaping.
        let intent = held.ticket().retry();
        let worker_lease = lease.clone();
        let source = scope.source.clone();
        let (space, frame) = (scope.space, scope.frame);
        let projection_scope = scope.projection_scope;
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = Arc::new(AtomicBool::new(false));
        let worker_ready = completed.clone();
        let input = (source, units, intent, worker_lease, signal, value);
        let thread = std::thread::Builder::new()
            .name("otter-projection-prepare".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(worker_ready, notify);
                // Convert captured inputs into later-declared local owners; normal/error/panic paths all clean up before publishing completion.
                let (source, units, intent, worker_lease, signal, value) = input;
                let _com = crate::desktop::Apartment::new()?;
                let check = || {
                    if signal.load(Ordering::Acquire) {
                        return Err(stale());
                    }
                    worker_lease
                        .validate(intent.origin(), now()?)
                        .map_err(|_| stale())
                };
                check()?;
                let frame = scope::placement::fit(
                    frame,
                    space,
                    (value.image().width, value.image().height),
                )?;
                let (restore, retained, expected) =
                    crate::desktop::session_candidate::projection::prepare(
                        value,
                        &intent,
                        projection_scope,
                        space,
                        frame,
                        crate::session_append::Target {
                            original: &source,
                            active: &units,
                        },
                        &signal,
                    )?;
                check()?;
                Ok(Candidate {
                    restore,
                    retained,
                    expected,
                })
            })
            .map_err(|error| Error::new(E_FAIL, format!("Cannot prepare projection: {error}")))?;
        Ok(Self {
            cancel,
            ready: completed,
            lease,
            held: Some(held),
            thread: Some(thread),
        })
    }
    pub(super) fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
        self.lease.revoke();
    }
    pub(super) fn stopped(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
    pub(super) fn cleanup_ticket(&self) -> Option<crate::projection_delivery::journal::Ticket> {
        self.held.as_ref()?.cleanup_ticket()
    }
    pub(super) fn expires_at_ms(&self) -> u64 {
        self.lease.expires_at_ms()
    }
    pub(super) fn valid(&self, origin: &str) -> bool {
        now().is_ok_and(|now| self.lease.validate(origin, now).is_ok())
    }
    pub(super) fn poll(&mut self) -> Option<Result<Prepared>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .thread
            .take()?
            .join()
            .unwrap_or_else(|_| Err(Error::new(E_FAIL, "Projection preparation panicked")));
        Some(result.map(|candidate| Prepared {
            restore: candidate.restore,
            retained: candidate.retained,
            expected: candidate.expected,
            held: self.held.take().unwrap(),
        }))
    }
    pub(super) fn reap_deadline(&self) -> Option<std::time::Instant> {
        reap_deadline(&self.ready)
    }
    #[cfg(test)]
    pub(super) fn finished(&self) -> bool {
        self.thread.as_ref().is_some_and(JoinHandle::is_finished)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // Normal transfer must not revoke the same Lease still owned by Hold; only uncollected threads use fallback cancellation.
        if self.thread.is_some() {
            self.stop();
            let _ = self.thread.take().unwrap().join();
        }
    }
}
pub(super) fn now() -> Result<u64> {
    crate::desktop::surface_windows::now_ms()
}
