//! Explicit session-image-directory selection; this dialog's owner reclaims both system path strings and COM objects.
use super::session_asset_root::Root;
use std::{os::windows::ffi::OsStringExt, path::PathBuf};
use windows::{
    core::{Result, HSTRING, PWSTR},
    Win32::{Foundation::*, System::Com::*, UI::Shell::*},
};
struct Name(PWSTR);
impl Drop for Name {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0 .0.cast()));
        }
    }
}
pub(super) unsafe fn choose(owner: HWND) -> Result<Option<Root>> {
    choose_with_title(owner, crate::ui_text::text("session-assets-title"))
}
pub(super) unsafe fn choose_workflow(owner: HWND) -> Result<Option<Root>> {
    choose_with_title(owner, crate::ui_text::text("session-archive-title"))
}
unsafe fn choose_with_title(owner: HWND, title: &str) -> Result<Option<Root>> {
    choose_path(owner, title)?
        .map(|p| Root::open(&p))
        .transpose()
}
pub(super) unsafe fn choose_path(owner: HWND, title: &str) -> Result<Option<PathBuf>> {
    let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
    dialog.SetOptions(
        dialog.GetOptions()?
            | FOS_PICKFOLDERS
            | FOS_FORCEFILESYSTEM
            | FOS_PATHMUSTEXIST
            | FOS_NOCHANGEDIR
            | FOS_DONTADDTORECENT,
    )?;
    dialog.SetTitle(&HSTRING::from(title))?;
    if let Err(error) = dialog.Show(Some(owner)) {
        if error.code() == windows::core::HRESULT::from_win32(ERROR_CANCELLED.0) {
            return Ok(None);
        }
        return Err(error);
    }
    let item = dialog.GetResult()?;
    let name = Name(item.GetDisplayName(SIGDN_FILESYSPATH)?);
    if name.0 .0.is_null() {
        return Err(windows::core::Error::new(
            E_INVALIDARG,
            "Missing selected asset directory",
        ));
    }
    let wide = name.0.as_wide();
    if wide.is_empty() || wide.len() > 32767 {
        return Err(windows::core::Error::new(
            E_INVALIDARG,
            "Invalid selected asset directory",
        ));
    }
    let path = PathBuf::from(std::ffi::OsString::from_wide(wide));
    // Explicit selection creates permission; never derive it from JSON or the session file's parent directory.
    Ok(Some(path))
}
