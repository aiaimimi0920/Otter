//! 默认桌面只读检查；线程 desktop 是借用句柄，input desktop 由本模块关闭。
use super::{Denied, Result};
use windows::Win32::{
    Foundation::HANDLE,
    System::{StationsAndDesktops::*, Threading::GetCurrentThreadId},
};
struct InputDesktop(HDESK);
impl Drop for InputDesktop {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseDesktop(self.0);
        }
    }
}
pub(super) fn check() -> Result<()> {
    let current =
        unsafe { GetThreadDesktop(GetCurrentThreadId()) }.map_err(|_| Denied::SecureDesktop)?;
    let input = InputDesktop(
        unsafe { OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) }
            .map_err(|_| Denied::SecureDesktop)?,
    );
    if !name(current)?.eq_ignore_ascii_case("default")
        || !name(input.0)?.eq_ignore_ascii_case("default")
    {
        return Err(Denied::SecureDesktop);
    }
    Ok(())
}
fn name(desktop: HDESK) -> Result<String> {
    let mut buffer = [0u16; 512];
    let mut needed = 0;
    unsafe {
        GetUserObjectInformationW(
            HANDLE(desktop.0),
            UOI_NAME,
            Some(buffer.as_mut_ptr().cast()),
            1024,
            Some(&mut needed),
        )
    }
    .map_err(|_| Denied::SecureDesktop)?;
    if !(2..=1024).contains(&needed) || needed % 2 != 0 {
        return Err(Denied::SecureDesktop);
    }
    let content = &buffer[..needed as usize / 2];
    if content.last() != Some(&0) {
        return Err(Denied::SecureDesktop);
    }
    String::from_utf16(&content[..content.len() - 1]).map_err(|_| Denied::SecureDesktop)
}
