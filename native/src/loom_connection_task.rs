//! 连接与断开共用的任务回收边界；完成通知不交付凭据，也不等于线程退出。
use super::{Cancel, Error};
use crate::desktop::loom_execution_signal::{reap_deadline, CompletionSignal, Notify};
use std::{
    io,
    sync::{atomic::AtomicBool, Arc},
    thread::JoinHandle,
    time::Instant,
};

pub(super) struct Task<T> {
    pub(super) cancel: Arc<Cancel>,
    ready: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<T, Error>>>,
}

// 关闭时才取消并同步回收；正常结算只领取已经退出的线程。
impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            self.cancel.cancel();
            let _ = thread.join();
        }
    }
}

impl<T: Send + 'static> Task<T> {
    pub(super) fn spawn(
        name: &str,
        notify: Notify,
        work: impl FnOnce(&Cancel) -> Result<T, Error> + Send + 'static,
    ) -> io::Result<Self> {
        let cancel = Arc::new(Cancel::new());
        let stop = cancel.clone();
        let ready = Arc::new(AtomicBool::new(false));
        let completed = ready.clone();
        let thread = std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                let _completion = CompletionSignal::new(completed, notify);
                // 清理登记的 socket；FnOnce 的凭据和连接局部资源也先于通知析构。
                struct Clear<'a>(&'a Cancel);
                impl Drop for Clear<'_> {
                    fn drop(&mut self) {
                        self.0.clear();
                    }
                }
                let _clear = Clear(&stop);
                work(&stop)
            })?;
        Ok(Self {
            cancel,
            ready,
            thread: Some(thread),
        })
    }
}

impl<T> Task<T> {
    pub(super) fn deadline(&self) -> Option<Instant> {
        self.thread.as_ref()?;
        reap_deadline(&self.ready)
    }
    pub(super) fn poll(&mut self) -> Option<Result<T, Error>> {
        if !self.thread.as_ref()?.is_finished() {
            return None;
        }
        Some(self.thread.take()?.join().unwrap_or(Err(Error::Worker)))
    }
    #[cfg(test)]
    pub(super) fn test_wait_finished(&self) {
        crate::desktop::loom_execution_tasks::reap_fixture::wait_finished(
            self.thread.as_ref().unwrap(),
        );
    }
}
