//! Ordinary captures settle immediately through original preparation-thread handles; closure exits nested pumps and application reclaims decoders.
use super::*;
use std::os::windows::io::AsRawHandle;
use windows::Win32::System::Threading::INFINITE;

pub(super) unsafe fn finish(hwnd: HWND, state: &State) -> Result<()> {
    let result = pump(hwnd, state);
    if result.is_err() {
        cancel(state);
    }
    result
}
unsafe fn pump(hwnd: HWND, state: &State) -> Result<()> {
    loop {
        if !IsWindow(Some(hwnd)).as_bool() {
            cancel(state);
            return Ok(());
        }
        let mut message = MSG::default();
        if PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            match message.message {
                WM_QUIT => {
                    // Leave the owner to the main loop or final shutdown.
                    cancel(state);
                    PostQuitMessage(message.wParam.0 as i32);
                    return Ok(());
                }
                WM_KEYDOWN if message.wParam.0 == 0x1b => cancel(state),
                _ => {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
        // Check completion after dispatching at most one message per round so continuous mouse messages cannot starve completed captures.
        if !IsWindow(Some(hwnd)).as_bool() {
            cancel(state);
            return Ok(());
        }
        super::finish(hwnd, state, false);
        let handle = state.open_job.borrow().as_ref().and_then(|job| {
            job.worker
                .handle
                .as_ref()
                .map(|worker| HANDLE(worker.as_raw_handle()))
        });
        let Some(handle) = handle else { return Ok(()) };
        // Reacquire handles before Peek/Dispatch; owners stay unchanged during OS waits, without model borrows.
        if MsgWaitForMultipleObjectsEx(Some(&[handle]), INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE)
            == WAIT_FAILED
        {
            return Err(Error::from_win32());
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "open_capture_ready_tests.rs"]
mod tests;
