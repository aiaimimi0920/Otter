//! Shared task-reclamation boundary for connect/disconnect; completion notices neither deliver credentials nor prove thread exit.
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

// Cancel and synchronously reclaim only during close; normal settlement collects only already-exited threads.
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
                // Clean up registered sockets; FnOnce credentials and connection-local resources are also destroyed before notification.
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
