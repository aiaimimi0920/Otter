//! Application-wide background notifications: at most one queued thread message at a time, with no idle timer.
use std::sync::{Arc, Mutex};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{PostThreadMessageW, WM_APP},
};

const MESSAGE: u32 = WM_APP + 92;
struct State {
    active: bool,
    pending: bool,
}
pub(in crate::desktop) struct Wake {
    state: Arc<Mutex<State>>,
    thread: u32,
}
impl Wake {
    /// The caller must already have created a window so the current UI thread has a message queue.
    pub(super) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                active: true,
                pending: false,
            })),
            thread: unsafe { GetCurrentThreadId() },
        }
    }
    pub(in crate::desktop) fn callback(&self) -> Arc<dyn Fn() + Send + Sync> {
        let state = self.state.clone();
        let thread = self.thread;
        Arc::new(move || {
            post(&state, || unsafe {
                PostThreadMessageW(thread, MESSAGE, WPARAM(0), LPARAM(0)).is_ok()
            });
        })
    }
    /// Clear pending before reading task mailboxes; concurrent updates are covered by this read or the next message.
    pub(super) fn acknowledge(&self) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).pending = false;
    }
    pub(super) fn close(&self) {
        // Exclude posting while closing; after close returns, old callbacks cannot post even if the thread ID is reused.
        self.state.lock().unwrap_or_else(|e| e.into_inner()).active = false;
    }
}
impl Drop for Wake {
    fn drop(&mut self) {
        self.close();
    }
}
fn post(state: &Mutex<State>, send: impl FnOnce() -> bool) {
    let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
    if state.active && !state.pending {
        // Failed posting does not occupy pending, allowing later notifications to retry; a full queue already contains messages that drive application polling.
        state.pending = send();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coalesces_retries_and_disables_old_callbacks() {
        let state = Mutex::new(State {
            active: true,
            pending: false,
        });
        post(&state, || false);
        assert!(!state.lock().unwrap().pending);
        post(&state, || true);
        post(&state, || panic!("duplicate wake"));
        state.lock().unwrap().pending = false;
        post(&state, || true);
        state.lock().unwrap().active = false;
        state.lock().unwrap().pending = false;
        post(&state, || panic!("closed wake"));
    }
    #[test]
    fn worker_wakes_real_ui_queue_without_input() {
        use windows::Win32::UI::WindowsAndMessaging::{PeekMessageW, MSG, PM_NOREMOVE, PM_REMOVE};
        let mut message = MSG::default();
        unsafe {
            let _ = PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_NOREMOVE);
        }
        let wake = Wake::new();
        let callback = wake.callback();
        std::thread::spawn(move || {
            for _ in 0..100 {
                callback();
            }
        })
        .join()
        .unwrap();
        assert!(unsafe { PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_REMOVE) }.as_bool());
        assert!(
            !unsafe { PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_REMOVE) }.as_bool()
        );
        wake.acknowledge();
        wake.callback()();
        assert!(unsafe { PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_REMOVE) }.as_bool());
        let late = wake.callback();
        drop(wake);
        late();
        assert!(
            !unsafe { PeekMessageW(&mut message, None, MESSAGE, MESSAGE, PM_REMOVE) }.as_bool()
        );
    }
}
