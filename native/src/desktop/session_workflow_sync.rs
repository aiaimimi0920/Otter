//! Single-slot per-item admission handshake between synchronous workers and UI; background work reads no window or Rc state.
use super::Ready;
use crate::{
    byte_budget::Budget, desktop::loom_execution_signal::CompletionSignal,
    loom_bridge_identity::Identity, session_document::workflow_snapshot::batch::dispatch,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

pub struct Admission {
    request_id: String,
    workflow_id: String,
    reply: SyncSender<bool>,
}
impl Admission {
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
    pub fn workflow_id(&self) -> &str {
        &self.workflow_id
    }
    /// Consume a one-shot response; expired/discarded requests cannot authorize the next message.
    pub fn resolve(self, allowed: bool) {
        let _ = self.reply.try_send(allowed);
    }
}
pub struct Delivery {
    pub ready: Ready,
    pub report: dispatch::Report,
}
pub struct Job {
    cancel: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
    admissions: Receiver<Admission>,
    worker: Option<JoinHandle<Delivery>>,
}
impl Job {
    pub fn start(
        ready: Ready,
        identity: Identity,
        budget: Arc<Budget>,
        limit: usize,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));
        let (tx, admissions) = mpsc::sync_channel(1);
        let signal = cancel.clone();
        let done = completed.clone();
        let worker = std::thread::Builder::new()
            .name("otter-workflow-sync".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(done, notify.clone());
                let report = match ready.batch(&signal) {
                    Ok(mut batch) => {
                        dispatch::run(&mut batch, &identity, &budget, limit, &signal, |request| {
                            admit(&tx, request.id(), request.workflow_id(), &signal, &notify)
                        })
                    }
                    Err(error) => dispatch::Report {
                        acknowledged: 0,
                        failure: Some(dispatch::Failure::Prepare(error)),
                        may_have_reached_remote: false,
                    },
                };
                Delivery { ready, report }
            })?;
        Ok(Self {
            cancel,
            completed,
            admissions,
            worker: Some(worker),
        })
    }
    pub fn take_admission(&self) -> Option<Admission> {
        self.admissions.try_recv().ok()
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Acquire)
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn deadline(&self) -> Option<Instant> {
        self.worker.as_ref()?;
        crate::desktop::loom_execution_signal::reap_deadline(&self.completed)
    }
    #[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
    pub(crate) fn test_wait_finished(&self) {
        crate::desktop::loom_execution_tasks::reap_fixture::wait_finished(
            self.worker.as_ref().unwrap(),
        );
    }
    /// Do not replace partial-success reports after cancellation with generic cancelled; retain remotely confirmed state.
    pub fn poll(&mut self) -> Option<Result<Delivery, &'static str>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .map_err(|_| "Workflow sync worker panicked"),
        )
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn admit(
    tx: &SyncSender<Admission>,
    request_id: &str,
    workflow_id: &str,
    cancel: &AtomicBool,
    notify: &Arc<dyn Fn() + Send + Sync>,
) -> bool {
    if cancel.load(Ordering::Acquire) {
        return false;
    }
    let (reply, rx) = mpsc::sync_channel(1);
    if tx
        .try_send(Admission {
            request_id: request_id.into(),
            workflow_id: workflow_id.into(),
            reply,
        })
        .is_err()
    {
        return false;
    }
    // Publish the mailbox before waking; notification panics and missing UI replies both reject rather than suspend workers forever.
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| notify())).is_err() {
        return false;
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if cancel.load(Ordering::Acquire) {
            return false;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        match rx.recv_timeout(remaining.min(Duration::from_millis(100))) {
            Ok(allowed) => return allowed && !cancel.load(Ordering::Acquire),
            Err(mpsc::RecvTimeoutError::Timeout) => (),
            Err(mpsc::RecvTimeoutError::Disconnected) => return false,
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_workflow_sync_tests.rs"]
mod tests;
