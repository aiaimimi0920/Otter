//! 输入子目标身份：限定原 root 的后代，持有原进程句柄，不把裸 HWND 当授权。
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
    /// 仅绑定 root 自身或现存后代；不替调用方枚举/重新命中，不授予显式控制权限。
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
    pub(crate) fn same_target(&self, other: &Self) -> bool {
        self.root.same_source(&other.root) && self.identity == other.identity
    }
    pub(crate) fn thread(&self) -> u32 {
        self.identity.thread
    }
    pub(crate) fn root(&self) -> &Target {
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
    /// 子窗口可属于不同进程，必须单独检查用户/session/UIPI；不能只检查 root。
    pub fn preflight(&self) -> Result<()> {
        self.validate()?;
        input_security::preflight(&self.root).map_err(Error::Security)?;
        input_security::check_process(HANDLE(self.process.as_raw_handle()))
            .map_err(Error::Security)?;
        self.validate()
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
// 与 root Target 一致：同一存活进程/线程内快速复用相同 HWND 仍有系统身份粒度边界。
#[cfg(test)]
#[path = "live_input_target_tests.rs"]
mod tests;
