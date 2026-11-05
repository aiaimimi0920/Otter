//! 单槽策略暂存 worker；发布仍在 UI 结算点，退出/丢弃只清理本任务暂存。
use super::file::{File, Prepared};
use crate::{
    desktop::loom_execution_signal::{CompletionSignal, Notify},
    projection_receive_policy::Change,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Prepared, String>>>,
}
impl Job {
    pub(super) fn start(file: File, change: Change, notify: Notify) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-receive-policy".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                file.prepare(change, &signal)
            })?;
        Ok(Self {
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub(super) fn poll(&mut self) -> Option<Result<Prepared, String>> {
        if !self.ready.load(Ordering::Acquire) && !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .unwrap_or_else(|_| Err("Receive policy worker panicked".into())),
        )
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
