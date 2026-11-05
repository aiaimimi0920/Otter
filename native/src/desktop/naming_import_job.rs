//! 单槽命名配置只读任务；容量准入在读文件前执行，退出取消并 join。
use super::loom_execution_signal::{CompletionSignal, Notify};
use crate::settings_source::Source;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    ready: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<Arc<Source>, String>>>,
}
impl Job {
    pub fn start(path: PathBuf, notify: Notify) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let ready = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let completed = ready.clone();
        let worker = std::thread::Builder::new()
            .name("otter-naming-import".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                let bytes = super::session_job::file::read_limited(
                    &path,
                    &signal,
                    crate::app_settings_document::MAX_BYTES,
                )
                .map_err(|e| e.to_string())?;
                Source::new(path, bytes)
            })?;
        Ok(Self {
            cancel,
            ready,
            worker: Some(worker),
        })
    }
    pub fn poll(&mut self) -> Option<Result<Arc<Source>, String>> {
        if !self.ready.load(Ordering::Acquire) && !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .map_err(|_| "Naming import worker panicked".into())
                .and_then(|result| result),
        )
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        // 操作系统阻塞读取不能强制中断；不遗留无 owner 后台线程。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
