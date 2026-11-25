//! STA waits pump only this thread's messages; cancel asynchronous operations, never poll on the UI thread.
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE,
};
pub(super) fn pump() {
    unsafe {
        let mut message = MSG::default();
        // Bound each message batch so message floods cannot delay cancellation checks.
        for _ in 0..32 {
            if !PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(2));
}
macro_rules! operation {
    ($operation:expr, $cancel:expr, $deadline:expr) => {{
        let cancel = $cancel;
        let deadline = $deadline;
        let operation = $operation;
        let mut stopped = None;
        loop {
            if stopped.is_none() {
                if let Err(error) = crate::clipboard_history::check(cancel, deadline) {
                    let _ = operation.Cancel();
                    stopped = Some(error);
                }
            }
            // Cancel is only a request. Keep the read permit until the operation leaves Started, preventing timed-out
            // menu reopenings from accumulating still-running provider operations; the UI owner must not block joining this thread.
            // windows-future 0.1.1 uses ABI value 0 for AsyncStatus::Started.
            if operation.Status()?.0 != 0 {
                break match stopped { Some(error) => Err(error), None => operation.GetResults() };
            }
            crate::clipboard_history::wait::pump();
        }
    }};
}
pub(super) use operation;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "clipboard_history_wait_tests.rs"]
mod tests;
