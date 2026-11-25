//! Menu-read tasks acquire a process permit before spawning; UI close only cancels, without waiting for the provider.
use super::{ReadPermit, Snapshot};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, TryRecvError},
    Arc,
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_FAIL,
};

/// Each task owns its result channel; results from a destroyed old menu cannot enter a new menu's task.
/// Consumers still verify window/document/menu target identity before applying results.
pub struct Task {
    cancel: Arc<AtomicBool>,
    receiver: Option<Receiver<Result<Snapshot>>>,
}

impl Task {
    pub(super) fn delete(id: String) -> Result<Self> {
        Self::spawn(move |cancel, permit| super::remove::remove(&id, cancel, permit))
    }
    pub fn start() -> Result<Self> {
        Self::spawn(super::read_with_permit)
    }

    pub(super) fn spawn(
        read: impl FnOnce(&AtomicBool, ReadPermit) -> Result<Snapshot> + Send + 'static,
    ) -> Result<Self> {
        // Reserve before spawn; blocking providers must not let repeated menu creation spawn unlimited threads.
        let permit = ReadPermit::acquire()?;
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("clipboard-history".into())
            .spawn(move || {
                let result = read(&worker_cancel, permit);
                if !worker_cancel.load(Ordering::Acquire) {
                    // One bounded result; if the receiver disappears, failed sending automatically releases its contents.
                    let _ = sender.send(result);
                }
            })
            .map_err(|_| Error::new(E_FAIL, "Clipboard history worker could not start"))?;
        Ok(Self {
            cancel,
            receiver: Some(receiver),
        })
    }

    /// Nonblocking, one-time result delivery. None means unfinished or already consumed/cancelled.
    pub fn poll(&mut self) -> Option<Result<Snapshot>> {
        let receiver = self.receiver.as_ref()?;
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(Error::new(
                E_FAIL,
                "Clipboard history worker ended without a result",
            )),
        };
        self.receiver.take();
        Some(result)
    }

    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Release);
        self.receiver.take();
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        // Do not join: the provider may ignore Cancel; its background thread retains the permit and COM resources.
        self.cancel();
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "clipboard_history_task_tests.rs"]
mod tests;
