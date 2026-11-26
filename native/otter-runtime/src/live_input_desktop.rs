//! Read-only default-desktop checks; thread desktop handles are borrowed, while this module closes the input desktop.
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
pub(crate) fn check() -> Result<()> {
    let current =
        unsafe { GetThreadDesktop(GetCurrentThreadId()) }.map_err(|_| Denied::SecureDesktop)?;
    let input = InputDesktop(
        unsafe { OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) }
            .map_err(|_| Denied::SecureDesktop)?,
    );
    if !is_default_desktop(current)? || !is_default_desktop(input.0)? {
        return Err(Denied::SecureDesktop);
    }
    Ok(())
}
/// Performance audit optimization:
/// Compare the UTF-16 desktop name against `"default"` directly on the stack instead of allocating two heap
/// `String`s (`String::from_utf16` + `eq_ignore_ascii_case`) on every pointer-move preflight (~30 heap allocs/move).
fn is_default_desktop(desktop: HDESK) -> Result<bool> {
    const EXPECTED: [u8; 7] = *b"default";
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
    let units = &content[..content.len() - 1];
    if units.len() != EXPECTED.len() {
        return Ok(false);
    }
    Ok(units
        .iter()
        .zip(EXPECTED)
        .all(|(&unit, expected)| unit <= 0x7f && (unit as u8).to_ascii_lowercase() == expected))
}
