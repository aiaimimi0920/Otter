//! Give known attachments one deactivation opportunity before exit; after the absolute deadline, original revocation/drain reaps without replaying unknown outcomes.
use super::*;
use std::time::{Duration, Instant};

const GRACE: Duration = Duration::from_secs(2);

impl Application {
    pub(in crate::desktop::application) fn finish_surface_transitions(&mut self) -> Result<()> {
        self.surface_windows.close_all()?;
        // Image GET needs no remote deactivation; cancel immediately to release shared slots for attachment lifecycle transactions.
        self.remote_sessions.cancel_surface_resources(|_, _| true);
        let deadline = Instant::now() + GRACE;
        pump_until(deadline, || {
            // Errors remain in Outcome and are reported by the final attachment gate after return to the window; no exit-time dialogs.
            let _ = self.remote_sessions.poll_surfaces();
            // closed admission records failure evidence only, without creating hidden windows for exit; late mounts can still deactivate.
            self.activate_surface_windows(false)?;
            Ok(self
                .remote_sessions
                .surface_shutdown_handles()
                .map(|handle| HANDLE(handle.as_raw_handle()))
                .chain(
                    self.surface_windows
                        .javascript_wait_handles()
                        .map(|handle| HANDLE(handle.as_raw_handle())),
                )
                .collect())
        })
    }
}

struct Quit(Option<i32>);
impl Drop for Quit {
    fn drop(&mut self) {
        if let Some(code) = self.0 {
            unsafe { PostQuitMessage(code) };
        }
    }
}

/// settle callbacks must keep returned handles alive until the next callback and never return handles of reaped tasks again.
/// Settle before blocking each turn, without timers or helper threads; messages do not reset the deadline.
fn pump_until(deadline: Instant, mut settle: impl FnMut() -> Result<Vec<HANDLE>>) -> Result<()> {
    let mut quit = Quit(None);
    loop {
        if Instant::now() >= deadline {
            return Err(Error::new(E_FAIL, "Surface shutdown grace period expired"));
        }
        let handles = settle()?;
        if handles.is_empty() {
            return Ok(());
        }
        let mut message = MSG::default();
        unsafe {
            if wait::next_until(&handles, &mut message, Some(deadline))? == 0 {
                // Remove WM_QUIT first and return it to original drain/outer handling at the end; never requeue it into a busy loop.
                quit.0 = Some(message.wParam.0 as i32);
            } else {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "application_surface_shutdown_tests.rs"]
mod tests;
