//! 有界 Loom 输入准备 worker；不创建执行 request，不修改文档，不发送网络消息。
use super::loom_execution_signal::{CompletionSignal, Notify};
use super::loom_inline::{self, Inline};
use crate::{
    editor::{DocumentStamp, Editor},
    pixels::Pixels,
    workspace_id::UnitId,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};
#[path = "loom_input_batch.rs"]
mod batch;
pub(crate) use batch::Freshness;
pub use batch::{PreparedBatch, Source};
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
const MAX_WORKERS: usize = 2;
struct Permit;
fn reserve_worker() -> Result<Permit, Error> {
    ACTIVE
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
            (count < MAX_WORKERS).then_some(count + 1)
        })
        .map_err(|_| Error::WorkerLimit)?;
    Ok(Permit)
}
impl Drop for Permit {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Debug)]
pub enum Error {
    Busy,
    WorkerLimit,
    InvalidInput,
    Budget,
    Allocation,
    Cancelled,
    Prepare(String),
    Spawn(std::io::Error),
    WorkerFailed,
}
struct Running {
    ready: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
    worker: JoinHandle<Result<PreparedBatch, Error>>,
}
#[derive(Default)]
pub struct Job {
    running: Option<Running>,
}
impl Job {
    pub fn is_busy(&self) -> bool {
        self.running.is_some()
    }
    pub fn start(
        &mut self,
        target: UnitId,
        target_editor: &Editor,
        sources: Vec<Source>,
    ) -> Result<(), Error> {
        self.start_with_notify(target, target_editor, sources, Arc::new(|| {}))
    }
    /// 通知只唤醒 UI owner，不得阻塞；发布完成前释放准备线程的并发许可。
    pub fn start_with_notify(
        &mut self,
        target: UnitId,
        target_editor: &Editor,
        sources: Vec<Source>,
        notify: Notify,
    ) -> Result<(), Error> {
        if self.is_busy() {
            return Err(Error::Busy);
        }
        batch::validate(&sources)?;
        if target_editor.draft().is_some() {
            return Err(Error::InvalidInput);
        }
        let permit = reserve_worker()?;
        let stamp = target_editor.document_stamp();
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let worker = thread::Builder::new()
            .name("otter-loom-input".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                let _permit = permit;
                batch::prepare(target, stamp, sources, &signal)
            })
            .map_err(Error::Spawn)?;
        self.running = Some(Running {
            ready,
            cancel,
            worker,
        });
        Ok(())
    }
    pub fn cancel(&self) {
        if let Some(running) = &self.running {
            running.cancel.store(true, Ordering::Release);
        }
    }
    pub fn poll(&mut self) -> Option<Result<PreparedBatch, Error>> {
        if !self.running.as_ref().is_some_and(|running| {
            running.ready.load(Ordering::Acquire) || running.worker.is_finished()
        }) {
            return None;
        }
        Some(self.join())
    }
    /// 最终退出时先取消再 join；正常交付使用完成通知和 poll。
    pub fn shutdown(&mut self) -> Option<Result<PreparedBatch, Error>> {
        self.running.as_ref()?;
        self.cancel();
        Some(self.join())
    }
    fn join(&mut self) -> Result<PreparedBatch, Error> {
        let running = self.running.take().unwrap();
        let result = running.worker.join().unwrap_or(Err(Error::WorkerFailed));
        if running.cancel.load(Ordering::Acquire) {
            Err(Error::Cancelled)
        } else {
            result
        }
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
#[cfg(test)]
#[path = "loom_input_job_tests.rs"]
mod tests;
#[cfg(test)]
pub(crate) use tests::SERIAL as TEST_SERIAL;
