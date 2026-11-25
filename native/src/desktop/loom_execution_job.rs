//! Bounded background Art tasks; updates coalesce into one slot, completed results retain pending-release leases, and no window access.
use super::{
    loom_execution_session::{Failure, Final, Session, Update},
    loom_input_job::PreparedBatch,
};
use crate::{
    graph_requests::Request, loom_bridge_identity::Identity, loom_bridge_stream::Interrupt,
    workspace_id::UnitId,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}
pub struct Execution {
    /// Local trusted installation/frozen parameters; not execute JSON and no replacement of remote identity/cleanup ownership.
    pub shader: Option<super::shader_prepare::Runtime>,
    pub identity: Identity,
    pub unit: UnitId,
    pub request: Request,
    pub message: String,
    pub inputs: PreparedBatch,
    pub timeout: Duration,
}
pub struct Completion {
    // Optional large contexts do not inflate every shared task's Completion enum; recipe data remains Arc-shared.
    pub(super) shader_recipe: Option<Box<super::shader_recipe_cache::Acquired>>,
    pub identity: Identity,
    pub unit: UnitId,
    pub request: Request,
    pub result: Result<Final, Failure>,
    pub remote_cancel:
        Option<Result<crate::loom_art_control::Outcome, crate::loom_art_control::Error>>,
    pub cleanup_retries: u8,
}
#[path = "loom_execution_cleanup.rs"]
mod cleanup;
use super::loom_execution_signal::{CompletionSignal, Notify};
#[derive(Debug)]
pub enum Error {
    Busy,
    InvalidTarget,
    WorkerLimit,
    Spawn(std::io::Error),
    WorkerFailed,
}
#[derive(Default)]
struct Mailbox {
    progress: Option<f64>,
    preview: Option<Update>,
    interrupt: Option<Interrupt>,
}
struct Running {
    ready: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    mailbox: Arc<Mutex<Mailbox>>,
    worker: JoinHandle<Completion>,
}
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
    completed_mailbox: Option<Arc<Mutex<Mailbox>>>,
}
impl Job {
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    /// Application owners handle registration, input freshness, and old-request cancellation before calling; failure must leave no active request.
    pub fn start(&mut self, execution: Execution) -> Result<(), Error> {
        self.start_with_notify(execution, Arc::new(|| {}))
    }
    /// Notifications only wake owners and must not access window models, block, or panic; notify updates after releasing mailbox locks.
    pub fn start_with_notify(
        &mut self,
        mut execution: Execution,
        notify: Notify,
    ) -> Result<(), Error> {
        let runtime = execution.shader.take();
        self.start_pipeline(execution, notify, runtime)
    }
    fn start_pipeline(
        &mut self,
        execution: Execution,
        notify: Notify,
        runtime: Option<super::shader_prepare::Runtime>,
    ) -> Result<(), Error> {
        if self.is_busy() {
            return Err(Error::Busy);
        }
        self.completed_mailbox = None;
        if execution.inputs.target != execution.unit {
            return Err(Error::InvalidTarget);
        }
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 2).then_some(count + 1)
            })
            .map_err(|_| Error::WorkerLimit)?;
        let permit = Permit;
        let cancel = Arc::new(AtomicBool::new(false));
        let mailbox = Arc::new(Mutex::new(Mailbox::default()));
        let signal = cancel.clone();
        let updates = mailbox.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let worker = thread::Builder::new()
            .name("otter-loom-execution".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify.clone());
                let _permit = permit;
                let inputs = execution.inputs;
                // Early cancellation must also release trusted bundles/other captured resources before completion signaling.
                let runtime = runtime;
                let session = Session {
                    identity: &execution.identity,
                    unit: &execution.unit,
                    request: &execution.request,
                    cancel: &signal,
                    timeout: execution.timeout,
                };
                let mut connected = false;
                let mut result = session.run(
                    execution.message,
                    |interrupt| {
                        connected = true;
                        let mut mailbox = updates.lock().unwrap_or_else(|error| error.into_inner());
                        if signal.load(Ordering::Acquire) {
                            let _ = interrupt.cancel();
                        }
                        mailbox.interrupt = Some(interrupt);
                    },
                    |update| {
                        if signal.load(Ordering::Acquire) {
                            return false;
                        }
                        let mut mailbox = updates.lock().unwrap_or_else(|error| error.into_inner());
                        match update {
                            Update::Progress(value) => mailbox.progress = Some(value),
                            preview => mailbox.preview = Some(preview),
                        }
                        drop(mailbox);
                        notify();
                        true
                    },
                );
                updates
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .interrupt = None;
                // Successful completion/explicit remote failure must not recancel; send local exceptions only if previously connected, never inventing cancellation.
                let remote_cancel = if connected
                    && result.is_err()
                    && !matches!(&result, Err(Failure::Remote(_)))
                {
                    Some(crate::loom_art_control::cancel_authenticated(
                        &execution.identity,
                        &execution.unit,
                        &execution.request,
                    ))
                } else {
                    None
                };
                // After successful terminal state, local failure/cancellation must not cancel completed remote-code retrieval; failed leases still use original compensation.
                let mut shader_recipe = None;
                if let (Ok(final_result), Some(runtime)) = (&mut result, runtime) {
                    match super::shader_fetch_render::render(
                        final_result,
                        &inputs,
                        runtime,
                        &signal,
                    ) {
                        Ok(recipe) => shader_recipe = recipe.map(Box::new),
                        Err(error) => result = Err(Failure::Shader(error)),
                    }
                }
                let mut completion = Completion {
                    shader_recipe,
                    identity: execution.identity,
                    unit: execution.unit,
                    request: execution.request,
                    result,
                    remote_cancel,
                    cleanup_retries: 0,
                };
                completion.retry_cleanup();
                completion
            })
            .map_err(Error::Spawn)?;
        self.running = Some(Running {
            ready,
            cancel,
            mailbox,
            worker,
        });
        Ok(())
    }
    pub fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
            let mut mailbox = running
                .mailbox
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(interrupt) = &mailbox.interrupt {
                let _ = interrupt.cancel();
            }
            mailbox.progress = None;
            mailbox.preview = None;
        }
    }
    /// Only previews with remote leases released enter the single slot; replacing old previews reclaims local pixels only.
    pub fn take_updates(&self) -> (Option<f64>, Option<Update>) {
        let Some(mailbox) = self
            .running
            .as_ref()
            .map(|running| &running.mailbox)
            .or(self.completed_mailbox.as_ref())
        else {
            return (None, None);
        };
        let mut mailbox = mailbox.lock().unwrap_or_else(|error| error.into_inner());
        if self
            .running
            .as_ref()
            .is_some_and(|running| running.cancel.load(Ordering::Acquire))
        {
            mailbox.progress = None;
            mailbox.preview = None;
        }
        (mailbox.progress.take(), mailbox.preview.take())
    }
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.running
            .as_ref()
            .and_then(|running| super::loom_execution_signal::reap_deadline(&running.ready))
    }
    /// Completion notification is not thread exit; UI never waits for notification tails, leaving continued reclamation to owner deadlines.
    pub fn poll(&mut self) -> Option<Result<Completion, Error>> {
        if !self
            .running
            .as_ref()
            .is_some_and(|running| running.worker.is_finished())
        {
            return None;
        }
        Some(self.join())
    }
    #[cfg(test)]
    pub(crate) fn test_wait_finished(&self) {
        super::loom_execution_tasks::reap_fixture::wait_finished(
            &self.running.as_ref().unwrap().worker,
        );
    }
    /// Application closure/task replacement must call explicitly and handle returned PendingRelease, not merely drop Job.
    pub fn shutdown(&mut self) -> Option<Result<Completion, Error>> {
        self.running.as_ref()?;
        self.cancel();
        Some(self.join())
    }
    fn join(&mut self) -> Result<Completion, Error> {
        let running = self.running.take().expect("running task");
        let mut completion = running.worker.join().map_err(|_| Error::WorkerFailed)?;
        if running.cancel.load(Ordering::Acquire) && completion.result.is_ok() {
            completion.result = Err(Failure::Cancelled);
        }
        if !running.cancel.load(Ordering::Acquire) {
            self.completed_mailbox = Some(running.mailbox);
        }
        Ok(completion)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        // The safety net prevents thread leaks; normal application paths must shutdown and transfer failed leases to retry owners.
        if let Some(Ok(completion)) = self.shutdown() {
            if completion.cleanup_pending() {
                eprintln!("otter-native: execution dropped with unresolved remote cleanup");
            }
        }
    }
}
