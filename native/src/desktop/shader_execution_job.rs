//! On-demand local Shader worker: prepare Image+PNG after the restricted child fully exits, without HWNDs or remote leases.
use super::shader_invocation::Invocation as OwnedInvocation;
use super::{
    loom_execution_signal::{CompletionSignal, Notify},
    loom_input_job::PreparedBatch,
    loom_output_batch::Batch,
};
use crate::{graph_requests::Request, workspace_id::UnitId};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};

pub struct Execution {
    pub unit: UnitId,
    pub request: Request,
    /// Callers decode/bind textures from these prepared inputs; credentials prove source freshness only, not that arbitrary textures share a source.
    pub inputs: PreparedBatch,
    pub shader: OwnedInvocation,
}
pub struct Completion {
    unit: UnitId,
    request: Request,
    batch: Option<Batch>,
}
impl Completion {
    pub(super) fn take(&mut self, unit: &UnitId, request: &Request) -> Option<Batch> {
        if &self.unit != unit || &self.request != request {
            return None;
        }
        self.batch.take()
    }
}
#[derive(Debug)]
pub enum Error {
    Busy,
    InvalidTarget,
    WorkerLimit,
    Admission(std::io::Error),
    Prepare(super::shader_payload::Error),
    Spawn(std::io::Error),
    Render(super::shader_invocation::Failure),
    Image(windows::core::Error),
    Output(super::loom_output_batch::Error),
    Cancelled,
    WorkerFailed,
}
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}
struct Running {
    ready: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    worker: JoinHandle<Result<Completion, Error>>,
}
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
}
impl Job {
    /// Cache hits are entirely local tasks; create no Session or remote Delivery and send no lease control.
    pub(super) fn start_cached(
        &mut self,
        mut execution: super::loom_execution_job::Execution,
        notify: Notify,
    ) -> Result<(), Error> {
        let runtime = execution
            .shader
            .take()
            .filter(|runtime| runtime.is_cached())
            .ok_or(Error::Prepare(super::shader_payload::Error::Admission))?;
        self.start_inputs(
            execution.unit,
            execution.request,
            execution.inputs,
            notify,
            move |inputs, cancel| {
                let invocation = runtime
                    .prepare_cached(inputs, cancel)
                    .map_err(Error::Prepare)?;
                let frame = invocation
                    .render(&|| cancel.load(Ordering::Acquire))
                    .map_err(Error::Render)?;
                drop(invocation);
                super::shader_image::prepare(frame, cancel)
                    .map_err(Error::Image)?
                    .into_batch()
                    .map_err(Error::Output)
            },
        )
    }
    pub(super) fn start(&mut self, execution: Execution, notify: Notify) -> Result<(), Error> {
        execution.shader.validate().map_err(Error::Admission)?;
        self.start_runner(
            execution.unit,
            execution.request,
            execution.inputs,
            notify,
            move |cancel| {
                let frame = execution
                    .shader
                    .render(&|| cancel.load(Ordering::Acquire))
                    .map_err(Error::Render)?;
                super::shader_image::prepare(frame, cancel)
                    .map_err(Error::Image)?
                    .into_batch()
                    .map_err(Error::Output)
            },
        )
    }
    // All production/test paths share permits, cancellation, completion fences, and join; runner replacement is only for deterministic lifecycle tests.
    pub(super) fn start_runner(
        &mut self,
        unit: UnitId,
        request: Request,
        inputs: PreparedBatch,
        notify: Notify,
        run: impl FnOnce(&AtomicBool) -> Result<Batch, Error> + Send + 'static,
    ) -> Result<(), Error> {
        self.start_inputs(unit, request, inputs, notify, move |_, cancel| run(cancel))
    }
    fn start_inputs(
        &mut self,
        unit: UnitId,
        request: Request,
        inputs: PreparedBatch,
        notify: Notify,
        run: impl FnOnce(&PreparedBatch, &AtomicBool) -> Result<Batch, Error> + Send + 'static,
    ) -> Result<(), Error> {
        if self.running.is_some() {
            return Err(Error::Busy);
        }
        if inputs.target != unit {
            return Err(Error::InvalidTarget);
        }
        ACTIVE
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                // Match the two slots in unified Tasks; actual children still serialize admission through shader_process's sole capacity pool.
                (count < 2).then_some(count + 1)
            })
            .map_err(|_| Error::WorkerLimit)?;
        let permit = Permit;
        let cancel = Arc::new(AtomicBool::new(false));
        let stopped = cancel.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let worker = thread::Builder::new()
            .name("otter-shader-execution".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                let _permit = permit;
                let prepared = inputs;
                // On early cancellation the runner is uncalled; still release its captured textures before publishing completion.
                let run = run;
                if stopped.load(Ordering::Acquire) {
                    return Err(Error::Cancelled);
                }
                let batch = run(&prepared, &stopped)?;
                if stopped.load(Ordering::Acquire) {
                    return Err(Error::Cancelled);
                }
                Ok(Completion {
                    unit,
                    request,
                    batch: Some(batch),
                })
            })
            .map_err(Error::Spawn)?;
        self.running = Some(Running {
            ready,
            cancel,
            worker,
        });
        Ok(())
    }
    pub(super) fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }
    pub(super) fn deadline(&self) -> Option<std::time::Instant> {
        self.running
            .as_ref()
            .and_then(|running| super::loom_execution_signal::reap_deadline(&running.ready))
    }
    pub(super) fn poll(&mut self) -> Option<Result<Completion, Error>> {
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
    pub(super) fn shutdown(&mut self) -> Option<Result<Completion, Error>> {
        self.running.as_ref()?;
        self.cancel();
        Some(self.join())
    }
    fn join(&mut self) -> Result<Completion, Error> {
        let running = self.running.take().expect("running Shader task");
        let result = running.worker.join().map_err(|_| Error::WorkerFailed)?;
        if running.cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        result
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
