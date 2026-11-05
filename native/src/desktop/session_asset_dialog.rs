//! 显式选择会话图片目录；系统路径字符串和 COM 对象均由本次对话框 owner 回收。
use super::session_asset_root::Root;
use std::{os::windows::ffi::OsStringExt, path::PathBuf};
use windows::{
    core::{w, Result, PWSTR},
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
    choose_with_title(owner, w!("选择本次工作区可读取的图片目录（仅直接子文件）"))
}
pub(super) unsafe fn choose_workflow(owner: HWND) -> Result<Option<Root>> {
    choose_with_title(
        owner,
        w!("选择工作流归档目录（写入图片，并启用保存后同步）"),
    )
}
unsafe fn choose_with_title(owner: HWND, title: windows::core::PCWSTR) -> Result<Option<Root>> {
    let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
    dialog.SetOptions(
        dialog.GetOptions()?
            | FOS_PICKFOLDERS
            | FOS_FORCEFILESYSTEM
            | FOS_PATHMUSTEXIST
            | FOS_NOCHANGEDIR
            | FOS_DONTADDTORECENT,
    )?;
    dialog.SetTitle(title)?;
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
    // 权限由显式选择创建，不从 JSON 或会话文件父目录推导。
    Root::open(&path).map(Some)
}
