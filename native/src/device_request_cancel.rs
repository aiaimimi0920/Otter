//! 单个串行设备授权任务的取消 owner；共享停止标志、当前 socket 与可唤醒等待。
use crate::loom_bridge_stream::Interrupt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Condvar, Mutex,
};
use std::time::Duration;
use std::{
    os::windows::io::{AsRawHandle, OwnedHandle},
    sync::Arc,
};
use windows::Win32::{Foundation::HANDLE, System::Threading::SetEvent};

#[derive(Default)]
pub struct Cancel {
    stop: AtomicBool,
    socket: Mutex<Option<Interrupt>>,
    wake: Condvar,
    native_wake: Mutex<Option<Arc<OwnedHandle>>>,
}
impl Cancel {
    #[cfg(test)]
    pub(crate) fn test_disconnect(&self) -> bool {
        self.socket
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|handle| handle.cancel().is_ok())
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }
    pub(super) fn flag(&self) -> &AtomicBool {
        &self.stop
    }
    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Release);
        let handle = self.socket.lock().unwrap_or_else(|e| e.into_inner()).take();
        self.wake.notify_all();
        if let Some(event) = self
            .native_wake
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            let _ = unsafe { SetEvent(HANDLE(event.as_raw_handle())) };
        }
        if let Some(handle) = handle {
            let _ = handle.cancel();
        }
    }
    /// 一个 owner 只允许一个串行请求链；登记与取消共用锁，避免握手前漏掉取消。
    pub(super) fn install(&self, handle: Interrupt) {
        let mut socket = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        if self.is_stopped() {
            let _ = handle.cancel();
        } else {
            *socket = Some(handle);
        }
    }
    /// 网络操作已返回后释放 clone，避免等待批准期间保留已完成请求的 socket。
    pub(super) fn clear(&self) {
        self.socket.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
    pub(super) fn install_wake(&self, event: Arc<OwnedHandle>) {
        let mut slot = self.native_wake.lock().unwrap_or_else(|e| e.into_inner());
        if self.is_stopped() {
            let _ = unsafe { SetEvent(HANDLE(event.as_raw_handle())) };
        }
        *slot = Some(event);
    }
    pub(super) fn clear_wake(&self) {
        self.native_wake
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
    }
    pub(super) fn wait(&self, duration: Duration) -> bool {
        let socket = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        // 与 cancel 的锁配合，不会在检查标志与进入等待之间丢失唤醒。
        let _guard = self
            .wake
            .wait_timeout_while(socket, duration, |_| !self.is_stopped())
            .unwrap_or_else(|e| e.into_inner());
        !self.is_stopped()
    }
}
impl Drop for Cancel {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_wait_is_immediate_and_repeated_cancel_is_safe() {
        let cancel = Cancel::new();
        cancel.cancel();
        cancel.cancel();
        let start = std::time::Instant::now();
        assert!(!cancel.wait(Duration::from_secs(30)));
        assert!(start.elapsed() < Duration::from_secs(1));
    }
    #[test]
    fn cancellation_wakes_a_waiter_without_periodic_polling() {
        let cancel = std::sync::Arc::new(Cancel::new());
        let worker_cancel = cancel.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            tx.send(()).unwrap();
            worker_cancel.wait(Duration::from_secs(30))
        });
        rx.recv().unwrap();
        cancel.cancel();
        assert!(!worker.join().unwrap());
    }
}
