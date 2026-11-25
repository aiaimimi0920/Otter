//! One-shot background capture owner; single result, irreversible cancellation, join on exit, no detached threads or idle polling.
use crate::capture_dynamic::{Mode, Outcome};
use std::{
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};
use windows::Win32::Foundation::RECT;
#[path = "capture_delivery.rs"]
mod delivery;
pub use delivery::{Captured, PreparedCapture};
#[path = "capture_context.rs"]
mod context;
pub(crate) use context::Context;

pub struct CaptureJob<T = Outcome> {
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<T, String>>>,
}
impl CaptureJob {
    /// Each object starts one capture only. The UI owner still bounds active tasks and rechecks document/window generations.
    pub fn start(screen: RECT, mode: Mode, standard_region: bool, gain: f32) -> io::Result<Self> {
        Self::spawn(move |cancel| {
            crate::desktop::capture::dynamic_region(screen, mode, standard_region, gain, cancel)
        })
    }
}
impl CaptureJob<PreparedCapture> {
    /// Capture and cache encoding share one worker; publish final files only after UI accepts PreparedCapture.
    pub fn start_cached(
        screen: RECT,
        mode: Mode,
        standard_region: bool,
        gain: f32,
        profile: std::path::PathBuf,
        stem: String,
        policy: crate::cache_policy::Policy,
    ) -> io::Result<Self> {
        Self::spawn(move |cancel| {
            let outcome = crate::desktop::capture::dynamic_region(
                screen,
                mode,
                standard_region,
                gain,
                cancel,
            )?;
            PreparedCapture::prepare(outcome, &profile, &stem, policy, cancel)
                .map_err(|error| error.to_string())
        })
    }
}
impl<T: Send + 'static> CaptureJob<T> {
    pub(crate) fn spawn(
        work: impl FnOnce(&AtomicBool) -> Result<T, String> + Send + 'static,
    ) -> io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = thread::Builder::new()
            .name("otter-capture".into())
            .spawn(move || {
                if signal.load(Ordering::Acquire) {
                    return Err("Capture cancelled".into());
                }
                let result = work(&signal);
                // Window close/user cancellation may occur inside an uninterruptible driver call; discard late results when it returns.
                if signal.load(Ordering::Acquire) {
                    Err("Capture cancelled".into())
                } else {
                    result
                }
            })?;
        Ok(Self {
            cancel,
            worker: Some(worker),
        })
    }
}
impl<T> CaptureJob<T> {
    /// Borrow the real thread handle for UI message waits; end the borrow before take_ready/Drop.
    pub fn completion_handle(&self) -> Option<std::os::windows::io::BorrowedHandle<'_>> {
        use std::os::windows::io::AsHandle;
        self.worker.as_ref().map(AsHandle::as_handle)
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Release);
    }
    pub fn pending(&self) -> bool {
        self.worker.is_some()
    }
    /// Return None immediately if unfinished. Extract results at most once; cancellation never returns a successful image.
    pub fn take_ready(&mut self) -> Option<Result<T, String>> {
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        let result = self
            .worker
            .take()?
            .join()
            .unwrap_or_else(|_| Err("Capture worker panicked".into()));
        if self.cancel.load(Ordering::Acquire) {
            Some(Err("Capture cancelled".into()))
        } else {
            Some(result)
        }
    }
}
impl<T> Drop for CaptureJob<T> {
    fn drop(&mut self) {
        self.cancel();
        // Do not hide leaks with detach; join may still await entered OS/driver calls, so UI must retain closing state.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_job_tests.rs"]
mod tests;
