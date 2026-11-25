//! Save-candidate worker; UI Receipt never enters the thread, and completion notification does not authorize commit.
use super::super::save_disk::{Destination, Disk};
use super::{Data, FrozenData};
use crate::{desktop::loom_execution_signal::CompletionSignal, session_write::Candidate};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub struct Output {
    pub checkpoint_accounts: Option<crate::projection_account::scene::Index>,
    pub checkpoint_senders: Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    pub checkpoint_scope: Option<crate::session_projection_scope::Proof>,
    pub checkpoint_projections: Option<crate::projection_delivery::binding::checkpoint::Index>,
    pub candidate: Candidate,
    pub staged: Option<Disk>,
    pub promote_host: bool,
}
impl Output {
    fn unstaged(candidate: Candidate) -> Self {
        Self {
            checkpoint_accounts: None,
            checkpoint_senders: None,
            checkpoint_scope: None,
            checkpoint_projections: None,
            candidate,
            staged: None,
            promote_host: false,
        }
    }
}
/// Library operations produce no disk staging or checkpoint; unchanged operations have no candidate bytes.
enum Completion {
    Prepared(Output),
    Library(Option<Candidate>),
}
/// Recycle candidates remove active windows and cannot be consumed by callers that only update library source content or save to disk.
enum Purpose {
    Save,
    Checkpoint,
    LibraryUpdate,
    Recycle(crate::workspace_id::UnitId),
}
#[path = "application_save_recycle_job.rs"]
mod recycle;
pub use recycle::Request as RecycleRequest;
#[path = "application_save_library_clear.rs"]
mod library_clear;
pub(in crate::desktop::application) struct ReferenceAddition {
    pub data: FrozenData,
    pub entry_id: String,
    pub created_at: String,
}

pub struct Job {
    prune_policy: Option<crate::session_library::recycle::policy::Policy>,
    purpose: Purpose,
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Completion, String>>>,
}
impl Job {
    pub fn prune_policy(&self) -> Option<crate::session_library::recycle::policy::Policy> {
        self.prune_policy
    }
    pub fn is_library_update(&self) -> bool {
        matches!(self.purpose, Purpose::LibraryUpdate)
    }
    pub fn is_save(&self) -> bool {
        matches!(self.purpose, Purpose::Save)
    }
    pub fn is_checkpoint(&self) -> bool {
        matches!(self.purpose, Purpose::Checkpoint)
    }
    pub fn start_checkpoint(
        mut data: Data,
        destination: Destination,
        revision: u64,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        data.disk_revision = Some(revision);
        let mut job = Self::start(data, Some(destination), notify)?;
        job.purpose = Purpose::Checkpoint;
        Ok(job)
    }
    pub fn recycle_unit(&self) -> Option<&crate::workspace_id::UnitId> {
        match &self.purpose {
            Purpose::Recycle(unit) => Some(unit),
            _ => None,
        }
    }
    pub(in crate::desktop::application) fn start_reference(
        original: crate::session_source::Source,
        unit: crate::workspace_id::UnitId,
        addition: Option<ReferenceAddition>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_library(notify, move |cancel| {
            let source = crate::session_document::decode(&original)
                .map_err(|error| format!("Invalid reference source: {error:?}"))?;
            let revision = source.revision;
            let encoded = addition
                .map(|addition| {
                    addition
                        .data
                        .encode(&source, &unit, cancel)
                        .map(|bytes| (bytes, addition.entry_id, addition.created_at))
                        .map_err(|error| format!("Reference snapshot: {error:?}"))
                })
                .transpose()?;
            use crate::session_library::reference::{prepare, Addition};
            let addition = encoded
                .as_ref()
                .map(|(snapshot, entry_id, created_at)| Addition {
                    snapshot,
                    entry_id,
                    created_at,
                });
            let bytes = prepare(&original, addition, &unit, cancel)?;
            Ok(Some(Candidate {
                bytes,
                expected_revision: revision,
                revision,
            }))
        })
    }
    pub fn start_library_remove(
        source: crate::session_source::Source,
        kind: crate::session_library::Kind,
        entry_id: String,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_library(notify, move |cancel| {
            crate::session_library::remove::prepare(&source, kind, &entry_id, cancel).map(Some)
        })
    }
    pub fn start(
        data: Data,
        destination: Option<Destination>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_with(notify, move |cancel| {
            let transient_host = data.transient_host.clone();
            let candidate = data
                .encode(cancel)
                .map_err(|error| format!("Save encoding failed: {error:?}"))?;
            // Encoding and disk staging share one cancellation token/task slot; the owner cleans up Staged after cancellation.
            let staged = destination
                .map(|destination| destination.prepare(&candidate, cancel))
                .transpose()
                .map_err(|error| format!("Save staging failed: {error}"))?;
            // All inclusion credentials share one session decode and bind the same bytes that will be published.
            let source = candidate.bytes.clone().into();
            let parsed = crate::session_source::Source::parse(&source).map_err(|_| {
                format!(
                    "Account projection checkpoint: {:?}",
                    crate::projection_account::Error::Response
                )
            })?;
            let checkpoint_scope = crate::session_projection_scope::Proof::capture_parsed(&parsed);
            let checkpoint_accounts = crate::projection_account::scene::Index::capture_parsed(
                &parsed,
                checkpoint_scope.as_ref(),
            )
            .map_err(|error| format!("Account projection checkpoint: {error:?}"))?;
            let indices = crate::projection_delivery::checkpoint_session::capture(&parsed);
            // Reuse the decoded candidate here, not on the HWND commit path.
            let promote_host =
                transient_host.is_some_and(|host| parsed.document().workspace.contains(&host));
            Ok(Output {
                checkpoint_accounts: Some(checkpoint_accounts),
                checkpoint_senders: indices.sender,
                checkpoint_scope,
                // The original Save candidate can actually persist through commit_new even if its destination is chosen later; library updates do not use this path.
                checkpoint_projections: indices.receiver,
                candidate,
                staged,
                promote_host,
            })
        })
    }
    fn start_with(
        notify: Arc<dyn Fn() + Send + Sync>,
        work: impl FnOnce(&AtomicBool) -> Result<Output, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        Self::start_worker(notify, move |cancel| work(cancel).map(Completion::Prepared))
    }
    fn start_library(
        notify: Arc<dyn Fn() + Send + Sync>,
        work: impl FnOnce(&AtomicBool) -> Result<Option<Candidate>, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let mut job =
            Self::start_worker(notify, move |cancel| work(cancel).map(Completion::Library))?;
        job.purpose = Purpose::LibraryUpdate;
        Ok(job)
    }
    fn start_worker(
        notify: Arc<dyn Fn() + Send + Sync>,
        work: impl FnOnce(&AtomicBool) -> Result<Completion, String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-session-save".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                work(&signal)
            })?;
        Ok(Self {
            prune_policy: None,
            purpose: Purpose::Save,
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn poll(&mut self) -> Option<Result<Output, String>> {
        Some(self.poll_completion()?.and_then(|result| match result {
            Completion::Prepared(output) => Ok(output),
            Completion::Library(_) => Err("Save result purpose mismatch".into()),
        }))
    }
    pub fn poll_library_update(&mut self) -> Option<Result<Option<Candidate>, String>> {
        Some(self.poll_completion()?.and_then(|result| match result {
            Completion::Library(change) => Ok(change),
            Completion::Prepared(_) => Err("Library result purpose mismatch".into()),
        }))
    }
    fn poll_completion(&mut self) -> Option<Result<Completion, String>> {
        if !self.is_ready() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Save worker panicked".into()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err("Save preparation cancelled".into())
        } else {
            result
        })
    }
    /// Recognize only actual thread exit, allowing ordinary and modal-gallery consumers to collect safely.
    pub fn is_ready(&self) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
    }
    pub fn deadline(&self) -> Option<std::time::Instant> {
        self.worker.as_ref()?;
        crate::desktop::loom_execution_signal::reap_deadline(&self.ready)
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        // WIC/font calls cannot be forcibly interrupted; wait for boundary cleanup on exit and never leave ownerless threads.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::desktop::loom_execution_tasks::reap_fixture::{wait_finished, Gate, WAIT};
    use crate::session_file_write::Staged;
    fn candidate() -> Candidate {
        Candidate {
            bytes: vec![1].into(),
            expected_revision: 1,
            revision: 2,
        }
    }
    #[test]
    fn completion_is_delivered_once_and_late_cancel_discards_ready_candidate() {
        for cancelled in [false, true] {
            let gate = Gate::new(|| {});
            let mut job =
                Job::start_with(gate.notify, |_| Ok(Output::unstaged(candidate()))).unwrap();
            gate.noticed.recv_timeout(WAIT).unwrap();
            assert!(!job.is_ready());
            assert!(job.poll().is_none());
            assert!(job.deadline().is_some());
            if cancelled {
                job.cancel();
            }
            gate.release.send(()).unwrap();
            wait_finished(job.worker.as_ref().unwrap());
            assert_eq!(job.poll().unwrap().is_err(), cancelled);
            assert!(job.poll().is_none());
            assert!(job.deadline().is_none());
        }
    }
    #[test]
    fn panic_notifies_and_drop_cancels_and_joins_worker() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut job = Job::start_with(
            Arc::new(move || {
                let _ = tx.send(());
            }),
            |_| panic!("save test panic"),
        )
        .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
        wait_finished(job.worker.as_ref().unwrap());
        assert_eq!(
            job.poll().unwrap().err().as_deref(),
            Some("Save worker panicked")
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let job = Job::start_with(Arc::new(|| {}), move |cancel| {
            while !cancel.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            tx.send(()).unwrap();
            Err("cancelled".into())
        })
        .unwrap();
        drop(job);
        rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
    }

    #[test]
    fn late_cancel_or_drop_reclaims_completed_staging_without_publishing() {
        for poll_cancelled in [false, true] {
            let fixture =
                crate::desktop::application::smoke_session_fixture::Directory::new().unwrap();
            let directory = fixture.path();
            let destination = fixture.target();
            let target = destination.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            let mut job = Job::start_with(
                Arc::new(move || { let _ = tx.send(()); }),
                move |cancel| {
                    let candidate = Candidate {
                        bytes: br#"{"documentSchemaVersion":1,"documentRevision":1,"stickers":[],"links":[]}"#.to_vec().into(),
                        expected_revision: 0,
                        revision: 1,
                    };
                    let staged = Staged::prepare(&target, &candidate, cancel)
                        .map_err(|error| error.to_string())?;
                    let mut output = Output::unstaged(candidate);
                    output.staged = Some(Disk::New(staged));
                    Ok(output)
                },
            ).unwrap();
            rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap();
            assert!(!destination.exists());
            assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
            if poll_cancelled {
                job.cancel();
                wait_finished(job.worker.as_ref().unwrap());
                assert!(job.poll().unwrap().is_err());
            }
            drop(job);
            assert!(!destination.exists());
            assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
            let directory = directory.to_owned();
            drop(fixture);
            assert!(!directory.exists());
        }
    }
}
