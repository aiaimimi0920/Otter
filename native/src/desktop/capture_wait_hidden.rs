//! Quick region capture only pumps messages, without flashing a wait dialog; the original task owner still reclaims cancellation/exit uniformly.
use super::*;
use std::os::windows::io::AsRawHandle;
use windows::Win32::System::Threading::INFINITE;

pub(super) unsafe fn run<T>(owner: HWND, input: &Input<T>) -> Result<(), String> {
    let template = layout::template();
    let window = crate::desktop::Window(
        CreateDialogIndirectParamW(
            None,
            template.as_ptr().cast(),
            Some(owner),
            Some(procedure::<T>),
            LPARAM(input as *const _ as isize),
        )
        .map_err(|error| error.to_string())?,
    );
    let mut message = MSG::default();
    while input.result.borrow().is_none() && IsWindow(Some(window.0)).as_bool() {
        // Collect messages before borrowing thread handles; Peek/Dispatch may reenter, so retain no task borrow across them.
        if PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
            if message.message == WM_QUIT {
                PostQuitMessage(message.wParam.0 as i32);
                input.job.borrow_mut().take();
                return Ok(());
            } else if message.message == WM_KEYDOWN && message.wParam.0 == 0x1b {
                cancel(window.0, input, None);
            } else {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        // Collect results promptly even with continuous input messages; do not wait for the queue to become completely idle.
        poll(window.0, input);
        if input.result.borrow().is_some() || !IsWindow(Some(window.0)).as_bool() {
            break;
        }
        let handle = input
            .job
            .borrow()
            .as_ref()
            .and_then(|job| {
                job.completion_handle()
                    .map(|handle| HANDLE(handle.as_raw_handle()))
            })
            .ok_or("Capture wait lost its pending worker")?;
        // This wait dispatches no window messages; end handle borrows before the next Peek while Input retains task ownership.
        if MsgWaitForMultipleObjectsEx(Some(&[handle]), INFINITE, QS_ALLINPUT, MWMO_INPUTAVAILABLE)
            == WAIT_FAILED
        {
            return Err(windows::core::Error::from_win32().to_string());
        }
    }
    Ok(())
}
