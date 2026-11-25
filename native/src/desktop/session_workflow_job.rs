//! Workflow-preparation worker: no window/network permissions; results imply neither save commit nor send authorization.
use super::*;
use crate::{
    desktop::{loom_execution_signal::CompletionSignal, session_asset_root::Root},
    session_source::Source,
};
use std::{sync::Arc, thread::JoinHandle};
#[path = "session_workflow_archive_job.rs"]
pub mod archive;
#[path = "session_workflow_sync.rs"]
pub mod sync;

pub struct Input {
    pub saved: Vec<u8>,
    pub revision: u64,
    // Pass only an explicitly admitted root owner; session paths must not expand access autonomously.
    pub root: Option<Arc<Root>>,
    pub catalog: Vec<serde_json::Value>,
}
pub struct Ready {
    prepared: Prepared,
    source: Source,
    metadata: workflow_snapshot::metadata::Plan,
}
impl Ready {
    pub fn batch(
        &self,
        cancel: &AtomicBool,
    ) -> std::result::Result<workflow_snapshot::batch::Batch<'_>, workflow_snapshot::Error> {
        workflow_snapshot::batch::Batch::from_plan(self.snapshot(), &self.metadata, cancel)
    }
    pub fn snapshot(&self) -> &Snapshot {
        self.prepared.snapshot()
    }
    pub fn metadata(&self) -> &workflow_snapshot::metadata::Plan {
        &self.metadata
    }
    pub fn matches_source(&self, saved: &[u8]) -> bool {
        self.metadata.matches(saved)
    }
    pub fn matches_retained_source(&self, saved: &Source) -> bool {
        // The same read-only owner already bound the digest in the worker; do not traverse the session again in the message pump.
        self.source.same_storage(saved) || self.matches_source(saved)
    }
}
fn prepare(
    source: Source,
    expected_revision: Option<u64>,
    root: Option<&Root>,
    catalog: &[serde_json::Value],
    cancel: &AtomicBool,
) -> std::result::Result<Ready, String> {
    let _apartment = crate::desktop::Apartment::new().map_err(|e| e.to_string())?;
    let candidate = prepare_source(&source, root, cancel).map_err(|e| e.to_string())?;
    if candidate.revision == 0 || expected_revision.is_some_and(|r| r != candidate.revision) {
        return Err("Workflow saved revision mismatch".into());
    }
    // Vectors, image-source selection, and snapshots share the borrowed document; candidates hold another handle to the same original text.
    let parsed = parse_source(&source, cancel).map_err(|e| e.to_string())?;
    let document = parsed.document();
    let candidate =
        VectorResources::prepare(&candidate.placements, &candidate.images, document, cancel)
            .map_err(|e| e.to_string())?
            .with_candidate(candidate);
    let prepared = candidate
        .prepare_workflow_snapshot_from(document, catalog, root, cancel)
        .map_err(|e| e.to_string())?;
    drop(candidate);
    let metadata = workflow_snapshot::metadata::prepare_from(prepared.snapshot(), document, cancel)
        .map_err(|e| format!("Workflow metadata preparation failed: {e:?}"))?;
    check(cancel).map_err(|e| e.to_string())?;
    Ok(Ready {
        prepared,
        source,
        metadata,
    })
}

pub struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<std::result::Result<Ready, String>>>,
}
impl Job {
    /// The UI only transfers the shared baseline; share original text and its budget, leaving parsing on the worker thread.
    pub fn start_shared(
        source: Source,
        root: Option<Arc<Root>>,
        catalog: Vec<serde_json::Value>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_with(notify, move |cancel| {
            prepare(source, None, root.as_deref(), &catalog, cancel)
        })
    }
    pub fn start(input: Input, notify: Arc<dyn Fn() + Send + Sync>) -> std::io::Result<Self> {
        Self::start_with(notify, move |cancel| {
            prepare(
                input.saved.into(),
                Some(input.revision),
                input.root.as_deref(),
                &input.catalog,
                cancel,
            )
        })
    }
    fn start_with(
        notify: Arc<dyn Fn() + Send + Sync>,
        work: impl FnOnce(&AtomicBool) -> std::result::Result<Ready, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-workflow-prepare".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                work(&signal)
            })?;
        Ok(Self {
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn deadline(&self) -> Option<std::time::Instant> {
        self.worker.as_ref()?;
        crate::desktop::loom_execution_signal::reap_deadline(&self.ready)
    }
    #[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
    pub(crate) fn test_wait_finished(&self) {
        crate::desktop::loom_execution_tasks::reap_fixture::wait_finished(
            self.worker.as_ref().unwrap(),
        );
    }
    pub fn poll(&mut self) -> Option<std::result::Result<Ready, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Workflow preparation worker panicked".into()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err("Workflow preparation cancelled".into())
        } else {
            result
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        // Native WIC calls are uninterruptible; the owner waits for cancellation-boundary reclamation, leaving no ownerless thread.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_workflow_job_tests.rs"]
mod tests;
