//! Original source-window state restoration; failures retain snapshots for owner retries, without autonomously deleting recovery records.
use super::Snapshot;
use std::os::windows::io::AsRawHandle;
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Threading::WaitForSingleObject, UI::WindowsAndMessaging::*},
};

impl Snapshot {
    /// Do not write windows if the original source is gone; query failures retain responsibility. Shared by local leases and independent recovery processes.
    pub fn restore_or_retire(&self) -> Result<()> {
        let target = &self.target;
        unsafe {
            match WaitForSingleObject(HANDLE(target.process_handle().as_raw_handle()), 0) {
                WAIT_OBJECT_0 => return Ok(()),
                WAIT_TIMEOUT => {}
                WAIT_FAILED => return Err(Error::from_win32()),
                _ => return Err(Error::new(E_FAIL, "Unexpected recovery source wait status")),
            }
            if !IsWindow(Some(target.hwnd())).as_bool() {
                return Ok(());
            }
            let mut process = 0;
            let thread = GetWindowThreadProcessId(target.hwnd(), Some(&mut process));
            if process == 0 || thread == 0 {
                return Err(Error::new(E_FAIL, "Recovery source identity unavailable"));
            }
            if process != target.identity.process || thread != target.identity.thread {
                return Ok(());
            }
        }
        self.restore()
    }
    /// Called only by owners holding visibility leases/recovery records; does not establish hiding transactions or replace the watchdog.
    /// Multiple Win32 calls are not an atomic transaction; return any failure to the owner, without clearing recovery records on that basis.
    pub fn restore(&self) -> Result<()> {
        let (width, height) = self.restore_dimensions()?;
        self.target.validate()?;
        unsafe {
            let hwnd = self.target.hwnd();
            SetLastError(ERROR_SUCCESS);
            let previous = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, self.extended_style);
            if previous == 0 && GetLastError() != ERROR_SUCCESS {
                return Err(Error::from_win32());
            }
            if let Some(layered) = self.layered {
                self.target.validate()?;
                SetLayeredWindowAttributes(hwnd, layered.color, layered.alpha, layered.flags)?;
            }
            self.target.validate()?;
            let order = if self.extended_style & WS_EX_TOPMOST.0 as isize != 0 {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            };
            SetWindowPos(
                hwnd,
                Some(order),
                self.rect.left,
                self.rect.top,
                width,
                height,
                SWP_NOACTIVATE | SWP_FRAMECHANGED,
            )?;
            self.target.validate()?;
            let mut placement = self.placement;
            // Match the baseline: expand an originally minimized source when the user recalls it, rather than minimizing it again.
            if !self.visible {
                // GetWindowPlacement's showCmd may not reflect WS_VISIBLE; do not accidentally show hidden sources.
                placement.showCmd = SW_HIDE.0 as u32;
            } else if self.iconic {
                placement.showCmd = SW_RESTORE.0 as u32;
            }
            SetWindowPlacement(hwnd, &placement)?;
            self.target.validate()
        }
    }
    fn restore_dimensions(&self) -> Result<(i32, i32)> {
        let width = i64::from(self.rect.right) - i64::from(self.rect.left);
        let height = i64::from(self.rect.bottom) - i64::from(self.rect.top);
        let layered = self.extended_style & WS_EX_LAYERED.0 as isize != 0;
        if width <= 0
            || height <= 0
            || width > i64::from(i32::MAX)
            || height > i64::from(i32::MAX)
            || self.placement.length as usize != std::mem::size_of::<WINDOWPLACEMENT>()
            || self.placement.showCmd > SW_FORCEMINIMIZE.0 as u32
            || layered != self.layered.is_some()
            || self.layered.is_some_and(|value| {
                value.flags.0 == 0 || value.flags.0 & !(LWA_ALPHA | LWA_COLORKEY).0 != 0
            })
        {
            return Err(Error::new(
                E_INVALIDARG,
                "Invalid Live window recovery snapshot",
            ));
        }
        Ok((width as i32, height as i32))
    }
}
