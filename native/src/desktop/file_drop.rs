//! Shell 文件拖入创建新钉图；先复制有界路径并释放 HDROP，再启动后台解码。
use super::State;
use std::{ffi::OsString, os::windows::ffi::OsStringExt, path::PathBuf};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        UI::{Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
};

struct DropFiles(HDROP);
impl Drop for DropFiles {
    fn drop(&mut self) {
        unsafe {
            DragFinish(self.0);
        }
    }
}

unsafe fn paths(drop: DropFiles) -> Result<Vec<PathBuf>> {
    let count = DragQueryFileW(drop.0, u32::MAX, None);
    if count == 0 || count as usize > super::open_job::MAX_BATCH {
        return Err(Error::new(
            E_INVALIDARG,
            "一次最多拖入 32 张图片，空批次不可导入。",
        ));
    }
    let mut paths = Vec::new();
    for index in 0..count {
        let length = DragQueryFileW(drop.0, index, None) as usize;
        if length == 0 || length > 32767 {
            return Err(Error::new(E_INVALIDARG, "拖入文件的路径为空或过长。"));
        }
        let mut name = vec![0u16; length + 1];
        if DragQueryFileW(drop.0, index, Some(&mut name)) as usize != length
            || name[length] != 0
            || name[..length].contains(&0)
        {
            return Err(Error::new(E_INVALIDARG, "无法读取完整的拖入文件路径。"));
        }
        paths.push(PathBuf::from(OsString::from_wide(&name[..length])));
    }
    Ok(paths)
}

pub(super) unsafe fn receive(hwnd: HWND, state: &State, parameter: WPARAM) {
    // 无论入口是否忙碌，都先接管并释放 Shell 包；不能泄漏重入投递的 HDROP。
    let dropped = DropFiles(HDROP(parameter.0 as *mut _));
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let result = paths(dropped).and_then(|paths| open(hwnd, state, paths));
    if let Err(error) = result {
        if IsWindow(Some(hwnd)).as_bool() {
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(format!("无法导入图片，原文档保留。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}

unsafe fn open(hwnd: HWND, state: &State, paths: Vec<PathBuf>) -> Result<()> {
    if state.open_job.borrow().is_some() {
        return Err(Error::new(
            E_FAIL,
            "图片仍在加载，请等待完成或按 Esc 取消。",
        ));
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return Ok(());
    }
    state.canvas.borrow_mut().cancel_gesture();
    if GetCapture() == hwnd {
        ReleaseCapture()?;
    }
    let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
    // 不在 UI 线程探测文件或解码；WIC 失败不修改原文档。
    super::open_job::start_destination(hwnd, state, super::open_job::Input::Files(paths), true)
}

#[cfg(test)]
#[path = "file_drop_tests.rs"]
mod tests;
