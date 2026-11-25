//! 有界 HKCU 适配器，仅访问 Otter 的 Run/StartupApproved 值；句柄作用域回收。
use super::*;
use windows::{
    core::{w, PCWSTR},
    Win32::{Foundation::ERROR_FILE_NOT_FOUND, System::Registry::*},
};
const RUN: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const APPROVED: PCWSTR =
    w!("Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run");
const NAME: PCWSTR = w!("Otter");
pub(super) struct Registry;
struct Key(HKEY);
impl Drop for Key {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}
fn read(path: PCWSTR, kind: REG_ROUTINE_FLAGS) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = vec![0u8; 4096];
    let mut length = bytes.len() as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            path,
            NAME,
            kind,
            None,
            Some(bytes.as_mut_ptr().cast()),
            Some(&mut length),
        )
    };
    if result == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    result.ok().map_err(io::Error::other)?;
    if length as usize > bytes.len() {
        return Err(io::Error::other("Startup value exceeds limit"));
    }
    bytes.truncate(length as usize);
    Ok(Some(bytes))
}
fn write(path: PCWSTR, kind: REG_VALUE_TYPE, bytes: &[u8]) -> io::Result<()> {
    let mut handle = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path,
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut handle,
            None,
        )
    }
    .ok()
    .map_err(io::Error::other)?;
    let key = Key(handle);
    unsafe { RegSetValueExW(key.0, NAME, None, kind, Some(bytes)) }
        .ok()
        .map_err(io::Error::other)
}
impl Store for Registry {
    fn command(&self) -> io::Result<Option<Vec<u8>>> {
        read(RUN, RRF_RT_REG_SZ)
    }
    fn approval(&self) -> io::Result<Option<Vec<u8>>> {
        read(APPROVED, RRF_RT_REG_BINARY)
    }
    fn set_command(&self, value: Option<&[u8]>) -> io::Result<()> {
        if let Some(bytes) = value {
            return write(RUN, REG_SZ, bytes);
        }
        let result = unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, RUN, NAME) };
        if result == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            result.ok().map_err(io::Error::other)
        }
    }
    fn approve(&self) -> io::Result<()> {
        write(APPROVED, REG_BINARY, &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
    }
}
