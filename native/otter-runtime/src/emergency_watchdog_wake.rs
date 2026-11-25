//! Zero-polling event wakeup for the independent emergency-exit child: a RawInput keyboard sink sleeps INFINITE while idle.
use super::{shutdown::Listener, Gate, Keys, SAMPLE_MS};
use crate::{live_recovery_parent::Parent, live_recovery_pipe::Pipe};
use std::{
    io,
    os::windows::io::AsRawHandle,
    sync::OnceLock,
    time::{Duration, Instant},
};
use windows::{
    core::{w, HRESULT},
    Win32::{
        Foundation::*,
        System::{LibraryLoader::GetModuleHandleW, Threading::*},
        UI::{
            Input::{RegisterRawInputDevices, RAWINPUTDEVICE, RIDEV_INPUTSINK, RIDEV_REMOVE},
            WindowsAndMessaging::*,
        },
    },
};

// Cover the full three-press Esc+Delete sequence with its 400ms inter-press window; immediately resume INFINITE sleep with zero wakeups when the sequence ends.
const ACTIVE_WINDOW: Duration = Duration::from_millis(1200);

struct Sink {
    hwnd: HWND,
}
impl Sink {
    unsafe fn new() -> windows::core::Result<Self> {
        register_class()?;
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("Otter.Native.EmergencySink"),
            w!(""),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(HINSTANCE(GetModuleHandleW(None)?.0)),
            None,
        )?;
        let device = RAWINPUTDEVICE {
            usUsagePage: 0x01,
            usUsage: 0x06,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: hwnd,
        };
        if let Err(error) =
            RegisterRawInputDevices(&[device], std::mem::size_of::<RAWINPUTDEVICE>() as u32)
        {
            let _ = DestroyWindow(hwnd);
            return Err(error);
        }
        // F12 is reserved for the system debugger; observe only physical state after RawInput, without claiming the global key combination.
        Ok(Self { hwnd })
    }
}
impl Drop for Sink {
    fn drop(&mut self) {
        unsafe {
            let device = RAWINPUTDEVICE {
                usUsagePage: 0x01,
                usUsage: 0x06,
                dwFlags: RIDEV_REMOVE,
                hwndTarget: HWND::default(),
            };
            let _ =
                RegisterRawInputDevices(&[device], std::mem::size_of::<RAWINPUTDEVICE>() as u32);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    DefWindowProcW(hwnd, msg, wp, lp)
}

unsafe fn register_class() -> windows::core::Result<()> {
    static REGISTERED: OnceLock<std::result::Result<(), HRESULT>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            let result = (|| -> windows::core::Result<()> {
                let class = WNDCLASSW {
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                    lpszClassName: w!("Otter.Native.EmergencySink"),
                    ..Default::default()
                };
                if RegisterClassW(&class) == 0 {
                    return Err(windows::core::Error::from_win32());
                }
                Ok(())
            })();
            result.map_err(|e| e.code())
        })
        .map_err(windows::core::Error::from_hresult)
}

pub(super) fn run_event_driven(
    parent: &Parent,
    pipe: &mut Pipe,
    mut keys: impl FnMut() -> Keys,
) -> io::Result<()> {
    let sink = unsafe { Sink::new().ok() };
    let mut listener = Listener::new(pipe)?;
    let mut gate = Gate::default();
    let mut active_until: Option<Instant> = None;
    let parent_handle = HANDLE(parent.handle().as_raw_handle());
    loop {
        let mut key_activity = false;
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == WM_INPUT {
                    key_activity = true;
                }
                DispatchMessageW(&msg);
            }
        }
        let now = Instant::now();
        listener.poll_signaled(now);
        if listener.expired(now) {
            parent.terminate_emergency().map_err(io::Error::other)?;
            return Ok(());
        }
        let must_sample = sink.is_none() || !gate.armed || key_activity || active_until.is_some();
        if must_sample {
            let state = keys();
            if gate.observe(state, now) {
                parent.terminate_emergency().map_err(io::Error::other)?;
                return Ok(());
            }
            if state.escape || state.delete || state.backup {
                active_until = Some(now + ACTIVE_WINDOW);
            } else if active_until.is_some_and(|end| now >= end) {
                active_until = None;
            }
        }
        let mut wait_ms = if sink.is_none() || !gate.armed || active_until.is_some() {
            SAMPLE_MS
        } else {
            INFINITE
        };
        if let Some(remaining) = listener.remaining(now) {
            if remaining.is_zero() {
                parent.terminate_emergency().map_err(io::Error::other)?;
                return Ok(());
            }
            let ms = remaining
                .as_millis()
                .saturating_add(1)
                .min(u128::from(INFINITE - 1)) as u32;
            wait_ms = wait_ms.min(ms);
        }
        let mut handles = [parent_handle, HANDLE::default()];
        let count = if let Some(pipe_event) = listener.wait_handle() {
            handles[1] = pipe_event;
            2
        } else {
            1
        };
        let status = unsafe {
            MsgWaitForMultipleObjectsEx(
                Some(&handles[..count]),
                wait_ms,
                QS_ALLINPUT,
                MWMO_INPUTAVAILABLE,
            )
        };
        if status == WAIT_OBJECT_0 {
            return Ok(());
        }
        if status == WAIT_FAILED {
            return Err(io::Error::last_os_error());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn raw_input_sink_creates_and_drops_cleanly_without_polling() {
        unsafe {
            let sink = Sink::new().unwrap();
            assert!(!sink.hwnd.is_invalid());
            drop(sink);
        }
    }
}
