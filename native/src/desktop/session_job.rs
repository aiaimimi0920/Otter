//! 单个会话准备任务 owner：完成通知、取消、回收；不自动发布到当前应用工作区。
use super::{
    loom_execution_signal::CompletionSignal,
    session_asset_root::Root,
    session_candidate::{self, VectorCandidate},
};
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

pub struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<VectorCandidate, String>>>,
}
impl Job {
    pub fn start(
        bytes: Vec<u8>,
        root: Option<Root>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> std::io::Result<Self> {
        Self::start_input(Input::Bytes(bytes), root, notify)
    }
    pub fn start_input(
        input: Input,
        root: Option<Root>,
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
                let (bytes, origin) = match input {
                    Input::Bytes(bytes) => (bytes, None),
                    Input::Frozen(input) => (input.prepare(&signal)?, None),
                    Input::File(path) => {
                        let (bytes, recovery) = file::read_recovering(&path, &signal)
                            .map_err(|error| error.to_string())?;
                        (
                            bytes,
                            Some(session_candidate::Origin {
                                requested: path,
                                recovery,
                            }),
                        )
                    }
                };
                let root = root;
                session_candidate::prepare(bytes, root.as_ref(), &signal)
                    .and_then(|mut candidate| {
                        candidate.origin = origin;
                        VectorCandidate::prepare(candidate, &signal)
                    })
                    .map_err(|error| error.to_string())
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
    pub fn poll(&mut self) -> Option<Result<VectorCandidate, String>> {
        if !self.ready.load(Ordering::Acquire) && !self.worker.as_ref()?.is_finished() {
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
        // 文件系统/WIC 调用不可强制中断；退出等待当前调用回收，不遗留无 owner 线程。
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
