//! Owner of one session-preparation task: completion notification, cancellation, and reclamation; no automatic publication into the current application workspace.
use super::{
    loom_execution_signal::CompletionSignal,
    session_asset_root::Root,
    session_candidate::{self, PreparedRestore},
};
use crate::loom_handshake::Handshake;
#[path = "session_file.rs"]
pub(super) mod file;
#[path = "session_frozen_input.rs"]
pub mod frozen;
pub enum Input {
    Bytes(Vec<u8>),
    File(std::path::PathBuf),
    Frozen(frozen::Input),
}
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub struct Ready {
    pub restore: PreparedRestore,
    pub(in crate::desktop) frozen: Option<frozen::Append>,
}

pub struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Ready, String>>>,
}
impl Job {
    #[cfg(test)]
    pub(crate) fn start(
        bytes: Vec<u8>,
        root: Option<Root>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_input(Input::Bytes(bytes), root, None, notify)
    }
    pub fn start_input(
        input: Input,
        root: Option<Root>,
        catalog: Option<Handshake>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        if matches!(&input, Input::Frozen(input) if !input.valid()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid frozen worker input",
            ));
        }
        if matches!(&input, Input::Bytes(bytes) if bytes.len() > crate::session_document::MAX_SESSION_BYTES)
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Session input exceeds budget",
            ));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-session-prepare".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                let (restore, frozen) = match input {
                    Input::Frozen(input) => {
                        drop(root); // Frozen instances do not use external asset directories.
                        let (restore, append) = input.prepare(catalog.as_ref(), &signal)?;
                        (restore, Some(append))
                    }
                    Input::Bytes(bytes) => (
                        PreparedRestore::prepare(bytes, root, catalog.as_ref(), &signal)
                            .map_err(|error| error.to_string())?,
                        None,
                    ),
                    Input::File(path) => {
                        let (bytes, recovery) = file::read_recovering(&path, &signal)
                            .map_err(|error| error.to_string())?;
                        let mut restore =
                            PreparedRestore::prepare(bytes, root, catalog.as_ref(), &signal)
                                .map_err(|error| error.to_string())?;
                        restore.vector.candidate.origin = Some(session_candidate::Origin {
                            requested: path,
                            recovery,
                        });
                        (restore, None)
                    }
                };
                Ok(Ready { restore, frozen })
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
        super::loom_execution_signal::reap_deadline(&self.ready)
    }
    #[cfg(test)]
    pub(crate) fn test_wait_finished(&self) {
        super::loom_execution_tasks::reap_fixture::wait_finished(self.worker.as_ref().unwrap());
    }
    pub fn poll(&mut self) -> Option<Result<Ready, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Session worker panicked".into()));
        Some(if self.cancel.load(Ordering::Acquire) {
            Err("Session preparation cancelled".into())
        } else {
            result
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel();
        // Filesystem/WIC calls cannot be forcibly interrupted; exit waits for the current call to be reclaimed, leaving no ownerless thread.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
#[path = "session_file_tests.rs"]
mod file_tests;
#[cfg(test)]
#[path = "session_job_tests.rs"]
mod tests;
