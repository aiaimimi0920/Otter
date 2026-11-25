//! Block on handles, window messages, or explicit deadlines; no timers, helper threads, or periodic polling.
use std::time::Instant;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Threading::INFINITE, UI::WindowsAndMessaging::*},
};

/// handles must remain valid and distinct throughout the call; accept only capture/restore task and process handles kept alive by the application.
pub(super) unsafe fn next(handles: &[HANDLE], message: &mut MSG) -> Result<i32> {
    next_until(handles, message, None)
}
/// The deadline is an absolute instant for this wait; spurious system wakeups must not keep extending authorization expiry.
pub(in crate::desktop::application) unsafe fn next_until(
    handles: &[HANDLE],
    message: &mut MSG,
    deadline: Option<Instant>,
) -> Result<i32> {
    if handles.len() > 63 {
        return Err(Error::new(
            E_INVALIDARG,
            "Too many application wait handles",
        ));
    }
    if handles.is_empty() && deadline.is_none() {
        let status = GetMessageW(message, None, 0, 0).0;
        return if status == -1 {
            Err(Error::from_win32())
        } else {
            Ok(status)
        };
    }
    loop {
        let timeout = match deadline {
            None => INFINITE,
            Some(deadline) => {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    return settle(message);
                }
                // Round up to avoid zero-timeout busy loops at submillisecond deadlines; reserve the special INFINITE value.
                remaining
                    .as_nanos()
                    .div_ceil(1_000_000)
                    .min((INFINITE - 1) as u128) as u32
            }
        };
        if PeekMessageW(message, None, 0, 0, PM_REMOVE).as_bool() {
            return Ok(i32::from(message.message != WM_QUIT));
        }
        let result =
            MsgWaitForMultipleObjectsEx(Some(handles), timeout, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        if result == WAIT_FAILED {
            return Err(Error::from_win32());
        }
        if result.0 < WAIT_OBJECT_0.0 + handles.len() as u32 {
            // Synthesize one ordinary settlement event without posting to the system message queue or waiting again on a completed thread.
            return settle(message);
        }
        // WAIT_TIMEOUT also returns to the absolute-deadline check, supporting u32-sized segments for very long waits.
        // System events may also wake us; Peek again rather than blocking on GetMessage and missing thread completion signals.
    }
}
fn settle(message: &mut MSG) -> Result<i32> {
    *message = MSG {
        message: WM_NULL,
        ..Default::default()
    };
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::windows::io::AsRawHandle, time::Duration};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    #[test]
    fn deadline_wakes_without_handles_or_posted_messages() {
        let start = Instant::now();
        let deadline = start + Duration::from_millis(20);
        let mut message = MSG::default();
        assert_eq!(
            unsafe { next_until(&[], &mut message, Some(deadline)) }.unwrap(),
            1
        );
        assert_eq!(message.message, WM_NULL);
        assert!(Instant::now() >= deadline);
    }
    #[test]
    fn expired_deadline_precedes_queued_input_without_consuming_it() {
        unsafe {
            let mut message = MSG::default();
            let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
            PostThreadMessageW(GetCurrentThreadId(), WM_APP + 998, WPARAM(9), LPARAM(0)).unwrap();
            assert_eq!(
                next_until(&[], &mut message, Some(Instant::now())).unwrap(),
                1
            );
            assert_eq!(message.message, WM_NULL);
            assert_eq!(next(&[], &mut message).unwrap(), 1);
            assert_eq!(message.message, WM_APP + 998);
        }
    }
    #[test]
    fn worker_completion_wakes_ui_without_posted_message() {
        let worker = std::thread::spawn(|| std::thread::sleep(Duration::from_millis(20)));
        let mut message = MSG::default();
        assert_eq!(
            unsafe { next(&[HANDLE(worker.as_raw_handle())], &mut message) }.unwrap(),
            1
        );
        assert_eq!(message.message, WM_NULL);
        assert!(worker.is_finished());
        worker.join().unwrap();
    }
    #[test]
    fn empty_wait_preserves_thread_messages_and_quit_code() {
        unsafe {
            let mut message = MSG::default();
            let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
            PostThreadMessageW(GetCurrentThreadId(), WM_APP + 999, WPARAM(17), LPARAM(0)).unwrap();
            assert_eq!(next(&[], &mut message).unwrap(), 1);
            assert_eq!((message.message, message.wParam.0), (WM_APP + 999, 17));
            PostQuitMessage(7);
            assert_eq!(next(&[], &mut message).unwrap(), 0);
            assert_eq!(message.wParam.0, 7);
        }
    }
}
