//! 一次只允许一个导出任务；像素通过 Arc 分享，完成检查仅在任务存在时启用。
use super::{export, State};
use std::{path::PathBuf, thread::JoinHandle};
use windows::{
    core::{w, HSTRING, PCWSTR},
    Win32::{
        Foundation::HWND,
        UI::{Controls::Dialogs::*, WindowsAndMessaging::*},
    },
};

pub(super) const EXPORT_TIMER: usize = 0x4F54;
#[path = "export_default.rs"]
mod default_save;
#[path = "export_original.rs"]
pub(super) mod original;
pub(super) use default_save::start as start_default;

pub(super) unsafe fn append(
    menu: HMENU,
    state: &State,
    command: usize,
) -> windows::core::Result<()> {
    let enabled =
        state.resolved_display.borrow().is_some() || state.canvas.borrow().image.is_some();
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        command,
        w!("导出 PNG…\tCtrl+S"),
    )
}

use super::file_dialog::Guard as Dialog;

fn snapshot_if_current(
    canvas: &super::canvas::Canvas,
    frame: Option<&super::display_frame::Frame>,
    stamp: &super::display_snapshot::Stamp,
) -> Option<(super::image::Image, crate::editor::Editor)> {
    if !stamp.matches(canvas, frame) {
        return None;
    }
    super::display_snapshot::capture(canvas, frame)
}

pub(super) struct ExportJob(Option<JoinHandle<Result<(), String>>>);
impl Drop for ExportJob {
    fn drop(&mut self) {
        // 不把未结束的文件写入泄漏成无人负责的后台任务。退出等待本次导出完成。
        if let Some(worker) = self.0.take() {
            let _ = worker.join();
        }
    }
}

pub(super) unsafe fn start(hwnd: HWND, state: &State) {
    if state.export.borrow().is_some()
        || (state.resolved_display.borrow().is_none() && state.canvas.borrow().image.is_none())
    {
        return;
    }
    let Some(_dialog) = Dialog::begin(&state.file_dialog) else {
        return;
    };
    state.canvas.borrow_mut().cancel_gesture();
    let stamp = super::display_snapshot::Stamp::capture(
        &state.canvas.borrow(),
        state.resolved_display.borrow().as_ref(),
    );
    let mut name = vec![0u16; 32768];
    if let Some(suggested) = super::export_naming::suggested(state) {
        for (destination, unit) in name.iter_mut().zip(suggested.encode_utf16()) {
            *destination = unit;
        }
    }
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: windows::core::PWSTR(name.as_mut_ptr()),
        nMaxFile: name.len() as u32,
        lpstrFilter: w!("PNG image\0*.png\0\0"),
        lpstrDefExt: w!("png"),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    if !GetSaveFileNameW(&mut options).as_bool() {
        if CommDlgExtendedError() != COMMON_DLG_ERRORS(0) {
            report(hwnd, "Cannot open export dialog");
        }
        return;
    }
    use std::os::windows::ffi::OsStringExt;
    if !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    let end = name
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(name.len());
    let path = PathBuf::from(std::ffi::OsString::from_wide(&name[..end]));
    let snapshot = snapshot_if_current(
        &state.canvas.borrow(),
        state.resolved_display.borrow().as_ref(),
        &stamp,
    );
    let Some((image, editor)) = snapshot else {
        report(hwnd, "选择保存路径期间文档已改变，未写入文件。请重新导出。");
        return;
    };
    let opacity = state.canvas.borrow().normal_opacity;
    spawn(hwnd, state, move || {
        export::save_new_with_opacity(&path, &image, &editor, opacity)
    });
}

unsafe fn spawn(
    hwnd: HWND,
    state: &State,
    operation: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    match std::thread::Builder::new()
        .name("otter-export".into())
        .spawn(operation)
    {
        Ok(worker) => {
            *state.export.borrow_mut() = Some(ExportJob(Some(worker)));
            if SetTimer(Some(hwnd), EXPORT_TIMER, 100, None) == 0 {
                // 无法调度完成回执时同步收尾，不能遗留无人观察的线程。
                finish(hwnd, state, true);
            }
        }
        Err(error) => report(hwnd, &format!("Cannot start export: {error}")),
    }
}

#[cfg(test)]
#[path = "export_job_tests.rs"]
mod tests;

pub(super) unsafe fn finish(hwnd: HWND, state: &State, force: bool) {
    let ready = state.export.borrow().as_ref().is_some_and(|job| {
        job.0
            .as_ref()
            .is_some_and(|thread| force || thread.is_finished())
    });
    if !ready {
        return;
    }
    let mut job = state.export.borrow_mut().take().unwrap();
    let _ = KillTimer(Some(hwnd), EXPORT_TIMER);
    let result = job
        .0
        .take()
        .unwrap()
        .join()
        .unwrap_or_else(|_| Err("Export worker failed".into()));
    if let Err(error) = result {
        report(hwnd, &error);
    }
}

unsafe fn report(hwnd: HWND, message: &str) {
    let message = HSTRING::from(message);
    MessageBoxW(
        Some(hwnd),
        PCWSTR(message.as_ptr()),
        w!("Otter export"),
        MB_OK | MB_ICONERROR,
    );
}
