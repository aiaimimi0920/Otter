//! 单个删除标记 worker；共用四槽与 Wake，不在 UI 触盘，不 detach 或提前归还容量。
use crate::{
    desktop::loom_execution_signal::{CompletionSignal, Notify},
    live_relay_control_runtime::Slot,
    projection_delivery::journal::{Error, Ticket},
};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};

pub(super) struct Job {
    thread: Option<JoinHandle<Result<Ticket, Error>>>,
    ready: Arc<AtomicBool>,
    _slot: Slot,
}
impl Job {
    pub(super) fn start(ticket: Ticket, notify: Notify) -> io::Result<Self> {
        let slot = Slot::acquire()?;
        let ready = Arc::new(AtomicBool::new(false));
        let signal = ready.clone();
        let thread = std::thread::Builder::new()
            .name("otter-projection-cleanup-mark".into())
            .spawn(move || {
                let _completion = CompletionSignal::new(signal, notify);
                let ticket = ticket; // 输入先于完成通知析构；成功 Ticket/许可移交给结果 owner。
                ticket.queue_cleanup()
            })?;
        Ok(Self {
            thread: Some(thread),
            ready,
            _slot: slot,
        })
    }
    pub(super) fn finished(&self) -> bool {
        self.ready.load(Ordering::Acquire) || self.thread.as_ref().unwrap().is_finished()
    }
    pub(super) fn finish(mut self) -> Result<Ticket, Error> {
        self.thread
            .take()
            .unwrap()
            .join()
            .unwrap_or(Err(Error::Write))
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join(); // 仅应用终态允许同步回收；正常路径先查 finished。
        }
    }
}
