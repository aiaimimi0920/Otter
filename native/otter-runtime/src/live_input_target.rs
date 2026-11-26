//! Input-subtarget identity: restricted to original-root descendants, retaining the original process handle, without treating a bare HWND as authorization.
use super::{input_security, Target};
use std::{
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::Arc,
};
use windows::Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    SourceChanged,
    NotDescendant,
    TargetChanged,
    PermissionDenied,
    Security(input_security::Denied),
}
type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Identity {
    window: usize,
    process: u32,
    thread: u32,
}
#[derive(Clone)]
pub struct Destination {
    root: Target,
    identity: Identity,
    process: Arc<OwnedHandle>,
}
impl Destination {
    /// Bind only root itself or an existing descendant; do not enumerate/re-hit-test for callers or grant explicit control permission.
    pub fn bind(root: &Target, window: HWND) -> Result<Self> {
        root.validate().map_err(|_| Error::SourceChanged)?;
        if window != root.hwnd() && !unsafe { IsChild(root.hwnd(), window) }.as_bool() {
            return Err(Error::NotDescendant);
        }
        let identity = identity(window)?;
        let process = unsafe {
            OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                identity.process,
            )
        }
        .map_err(|_| Error::PermissionDenied)?;
        let result = Self {
            root: root.clone(),
            identity,
            process: Arc::new(unsafe { OwnedHandle::from_raw_handle(process.0) }),
        };
        result.preflight()?;
        Ok(result)
    }
    pub fn hwnd(&self) -> HWND {
        HWND(self.identity.window as *mut _)
    }
    pub fn same_target(&self, other: &Self) -> bool {
        self.root.same_source(&other.root) && self.identity == other.identity
    }
    pub fn thread(&self) -> u32 {
        self.identity.thread
    }
    pub fn root(&self) -> &Target {
        &self.root
    }
    pub fn validate(&self) -> Result<()> {
        self.root.validate().map_err(|_| Error::SourceChanged)?;
        if unsafe { WaitForSingleObject(HANDLE(self.process.as_raw_handle()), 0) } != WAIT_TIMEOUT
            || identity(self.hwnd())? != self.identity
        {
            return Err(Error::TargetChanged);
        }
        if self.hwnd() != self.root.hwnd()
            && !unsafe { IsChild(self.root.hwnd(), self.hwnd()) }.as_bool()
        {
            return Err(Error::NotDescendant);
        }
        Ok(())
    }
    /// Child windows may belong to different processes and need separate user/session/UIPI checks; checking root alone is insufficient.
    pub fn preflight(&self) -> Result<()> {
        self.validate()?;
        // Performance audit optimization:
        // `self.process` was already opened with `PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION` and verified
        // alive in `self.validate()`. When the child belongs to the same PID as `self.root`, checking `self.process`
        // plus bracketing `desktop::check()` validates both root and child process tokens without opening and querying
        // the exact same process token a second time. Cross-process children still check both processes independently.
        if self.identity.process == self.root.process_id() {
            input_security::desktop::check().map_err(Error::Security)?;
            input_security::check_process_tokens(HANDLE(self.process.as_raw_handle()))
                .map_err(Error::Security)?;
            self.validate()?;
            input_security::desktop::check().map_err(Error::Security)
        } else {
            input_security::preflight(&self.root).map_err(Error::Security)?;
            input_security::check_process_tokens(HANDLE(self.process.as_raw_handle()))
                .map_err(Error::Security)?;
            self.validate()
        }
    }
}
fn identity(window: HWND) -> Result<Identity> {
    if !unsafe { IsWindow(Some(window)) }.as_bool() {
        return Err(Error::TargetChanged);
    }
    let mut process = 0;
    let thread = unsafe { GetWindowThreadProcessId(window, Some(&mut process)) };
    if process == 0 || thread == 0 {
        return Err(Error::TargetChanged);
    }
    Ok(Identity {
        window: window.0 as usize,
        process,
        thread,
    })
}
// As with root Target, rapid reuse of the same HWND within a live process/thread still has a system identity-granularity limit.
#[cfg(test)]
#[path = "live_input_target_tests.rs"]
mod tests;
