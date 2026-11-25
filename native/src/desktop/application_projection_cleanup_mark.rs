//! 单个删除标记 worker；共用四槽与 Wake，不在 UI 触盘，不 detach 或提前归还容量。
use crate::{
    desktop::loom_execution_signal::{reap_deadline, CompletionSignal, Notify},
    live_relay_control_runtime::Slot,
    projection_delivery::journal::{Error, Ticket},
};
use std::{
    io,
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
    time::Instant,
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
        self.thread.as_ref().unwrap().is_finished()
    }
    pub(super) fn reap_deadline(&self) -> Option<Instant> {
        reap_deadline(&self.ready)
    }
    pub(super) fn finish(mut self) -> Result<Ticket, Error> {
        self.thread
            .take()
            .unwrap()
            .join()
            .unwrap_or(Err(Error::Write))
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
mod tests {
    use super::*;
    use crate::projection_delivery::{
        accept::tests::invitation,
        journal::{tests::Directory, Store},
    };
    use std::{sync::mpsc, time::Duration};

    #[test]
    fn projection_cleanup_mark_notification_does_not_make_running_worker_joinable() {
        let _serial = crate::loom_bridge_client::tests::execution_job::SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let directory = Directory::new();
        let store = Store::open(&directory.0).unwrap();
        let ticket = store
            .begin(&invitation("https://loom.example"), "device-old")
            .unwrap();
        let (notified, notification) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let blocked = std::sync::Mutex::new(blocked);
        let job = Job::start(
            ticket,
            Arc::new(move || {
                notified.send(()).unwrap();
                let _ = blocked.lock().unwrap().recv();
            }),
        )
        .unwrap();
        let release = release; // 若断言失败，先释放通知，再让 Job 的 Drop 回收。
        notification.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!job.finished());
        assert!(job.reap_deadline().is_some());
        assert!(store.pending().unwrap()[0].cleanup_pending());
        release.send(()).unwrap();
        assert!(job.finish().unwrap().cleanup_pending());
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            let _ = thread.join(); // 仅应用终态允许同步回收；正常路径先查 finished。
        }
    }
}
