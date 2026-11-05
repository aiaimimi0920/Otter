//! 原始捕获另存，不渲染标注/裁剪；复用导出单任务 owner 与退出 join。
use super::*;
use std::{
    fs::File,
    os::windows::ffi::OsStringExt,
    sync::{atomic::AtomicBool, Arc},
};
use windows::core::PWSTR;
pub(in crate::desktop) const COMMAND: usize = 0x1069;
fn source(state: &State) -> Option<Arc<File>> {
    if state.resolved_display.borrow().is_some() {
        return None;
    }
    state
        .canvas
        .borrow()
        .capture_source()?
        .original_lease
        .clone()
}
pub(in crate::desktop) unsafe fn append(menu: HMENU, state: &State) -> windows::core::Result<()> {
    let enabled = state.export.borrow().is_none() && source(state).is_some();
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        COMMAND,
        w!("另存 HDR 原图（不含编辑）…"),
    )
}
pub(in crate::desktop) unsafe fn start(hwnd: HWND, state: &State) {
    if state.export.borrow().is_some() || state.modals.blocked() {
        return;
    }
    let Some(lease) = source(state) else {
        return;
    };
    let Some(_dialog) = Dialog::begin(&state.file_dialog) else {
        return;
    };
    let stamp = state.canvas.borrow().editor.document_stamp();
    let mut name = vec![0u16; 32768];
    for (out, unit) in name.iter_mut().zip("Otter_HDR_original.png".encode_utf16()) {
        *out = unit;
    }
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: PWSTR(name.as_mut_ptr()),
        nMaxFile: name.len() as u32,
        lpstrFilter: w!("HDR PNG\0*.png\0\0"),
        lpstrDefExt: w!("png"),
        lpstrTitle: w!("另存 HDR 原图（不含编辑）"),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    if !GetSaveFileNameW(&mut options).as_bool() {
        if CommDlgExtendedError() != COMMON_DLG_ERRORS(0) {
            report(hwnd, "无法打开 HDR 原图保存对话框。");
        }
        return;
    }
    if !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    if !stamp.matches(&state.canvas.borrow().editor)
        || !source(state).is_some_and(|current| Arc::ptr_eq(&current, &lease))
    {
        report(hwnd, "选择路径期间截图来源已改变，未写入文件。");
        return;
    }
    let end = name
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(name.len());
    let path = PathBuf::from(std::ffi::OsString::from_wide(&name[..end]));
    spawn(hwnd, state, move || {
        let cancel = AtomicBool::new(false);
        crate::capture_file::prepare_original(&lease, &path, &cancel)
            .and_then(|staged| staged.commit_new(&cancel))
            .map(|_| ())
            .map_err(|error| format!("无法另存 HDR 原图；不会覆盖已有文件。\n{error}"))
    });
}
