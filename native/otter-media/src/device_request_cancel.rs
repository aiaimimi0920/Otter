//! Cancellation owner for one serial device-authorization task; shared stop flag/current socket/wakeable waits.
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
    #[cfg(any(test, feature = "test-support"))]
    pub fn test_disconnect(&self) -> bool {
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
    pub fn flag(&self) -> &AtomicBool {
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
    /// One serial request chain per owner; registration/cancellation share a lock to prevent missed cancellation before handshakes.
    pub fn install(&self, handle: Interrupt) {
        let mut socket = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        if self.is_stopped() {
            let _ = handle.cancel();
        } else {
            *socket = Some(handle);
        }
    }
    /// Release clones after network operations return, avoiding completed-request sockets retained during approval waits.
    pub fn clear(&self) {
        self.socket.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
    pub fn install_wake(&self, event: Arc<OwnedHandle>) {
        let mut slot = self.native_wake.lock().unwrap_or_else(|e| e.into_inner());
        if self.is_stopped() {
            let _ = unsafe { SetEvent(HANDLE(event.as_raw_handle())) };
        }
        *slot = Some(event);
    }
    pub fn clear_wake(&self) {
        self.native_wake
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
    }
    pub fn wait(&self, duration: Duration) -> bool {
        let socket = self.socket.lock().unwrap_or_else(|e| e.into_inner());
        // Coordinate with cancel's lock to prevent lost wakeups between flag checks and waits.
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
