//! Background archival stage: asset publication, session assembly, and exact-baseline staging; commit authority remains with the UI Receipt owner.
use super::{CompletionSignal, Ready};
use crate::{
    desktop::session_asset_root::{
        archive::{self, Prepared},
        Root,
    },
    session_file_replace::{Failure, Published, Replacement},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub struct Staged {
    // Clean session staging/release the baseline lock before returning asset handles and the directory owner.
    replacement: Replacement,
    prepared: Prepared,
    root: Arc<Root>,
}
pub struct Committed {
    pub prepared: Prepared,
    pub publication: Published,
    pub root: Arc<Root>,
}
impl Staged {
    pub fn candidate(&self) -> &crate::session_write::Candidate {
        self.prepared.candidate()
    }
    /// Callers must execute after UI Receipt verification without pumping messages, and swap the saved baseline only on success.
    /// Do not discard returned recovery-file information; failure does not delete already published image assets.
    pub fn commit(self, cancel: &AtomicBool) -> Result<Committed, Failure> {
        let publication = self.replacement.commit(cancel)?;
        Ok(Committed {
            prepared: self.prepared,
            publication,
            root: self.root,
        })
    }
}
pub struct Job {
    cancel: Arc<AtomicBool>,
    completed: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Staged, String>>>,
}
/// Archives do not send image snapshots; the original-text owner retains its byte budget rather than returning it early with baked snapshots.
struct ArchiveInput {
    source: crate::session_source::Source,
    metadata: crate::session_document::workflow_snapshot::metadata::Plan,
}
impl ArchiveInput {
    fn from_ready(ready: Ready) -> Self {
        let Ready {
            prepared,
            source,
            metadata,
        } = ready;
        // Release in the worker so the UI does not destroy large stale snapshots; a new revision will prepare again.
        drop(prepared);
        Self { source, metadata }
    }
}
impl Job {
    pub fn start(
        ready: Ready,
        root: Arc<Root>,
        destination: PathBuf,
        timestamp_ms: u64,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let done = completed.clone();
        let worker = std::thread::Builder::new()
            .name("otter-workflow-archive".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(done, notify);
                if signal.load(Ordering::Acquire) {
                    return Err("Workflow archive cancelled".into());
                }
                let input = ArchiveInput::from_ready(ready);
                let batch =
                    archive::prepare(&root, &input.metadata, &input.source, timestamp_ms, &signal)
                        .map_err(|_| "Workflow resource archive failed")?;
                let prepared = batch
                    .into_candidate(&signal)
                    .map_err(|_| "Workflow archive session assembly failed")?;
                let replacement = Replacement::prepare(
                    &destination,
                    &input.source,
                    prepared.candidate(),
                    &signal,
                )
                .map_err(|_| "Workflow archive baseline or staging failed")?;
                Ok(Staged {
                    replacement,
                    prepared,
                    root,
                })
            })?;
        Ok(Self {
            cancel,
            completed,
            worker: Some(worker),
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn deadline(&self) -> Option<std::time::Instant> {
        self.worker.as_ref()?;
        crate::desktop::loom_execution_signal::reap_deadline(&self.completed)
    }
    pub fn poll(&mut self) -> Option<Result<Staged, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Workflow archive worker panicked".into()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err("Workflow archive cancelled".into())
        } else {
            result
        })
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_workflow_archive_job_tests.rs"]
mod tests;
