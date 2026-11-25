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

pub(super) struct ExportJob {
    worker: Option<JoinHandle<Result<(), String>>>,
    _permit: crate::edit_worker_budget::Permit<'static>,
}
impl ExportJob {
    pub(super) fn start(
        operation: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> Result<Self, String> {
        let permit = crate::edit_worker_budget::Permit::acquire_output()
            .ok_or("Native output worker limit reached")?;
        let worker = std::thread::Builder::new()
            .name("otter-export".into())
            .spawn(operation)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            worker: Some(worker),
            _permit: permit,
        })
    }

    pub(super) fn is_finished(&self) -> bool {
        self.worker.as_ref().is_some_and(JoinHandle::is_finished)
    }

    pub(super) fn complete(mut self) -> Result<(), String> {
        self.worker
            .take()
            .unwrap()
            .join()
            .unwrap_or_else(|_| Err("Export worker failed".into()))
    }
}
impl Drop for ExportJob {
    fn drop(&mut self) {
        // 不把未结束的文件写入泄漏成无人负责的后台任务。退出等待本次导出完成。
        if let Some(worker) = self.worker.take() {
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
    let style = export::style::Snapshot::capture(&state.canvas.borrow());
    spawn(hwnd, state, move || {
        style.save_new(&path, &image, &editor, opacity)
    });
}

unsafe fn spawn(
    hwnd: HWND,
    state: &State,
    operation: impl FnOnce() -> Result<(), String> + Send + 'static,
) {
    if SetTimer(Some(hwnd), EXPORT_TIMER, 100, None) == 0 {
        report(hwnd, "Cannot schedule export result");
        return;
    }
    match ExportJob::start(operation) {
        Ok(job) => *state.export.borrow_mut() = Some(job),
        Err(error) => {
            let _ = KillTimer(Some(hwnd), EXPORT_TIMER);
            report(hwnd, &format!("Cannot start export: {error}"));
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "export_job_tests.rs"]
mod tests;

pub(super) unsafe fn finish(hwnd: HWND, state: &State) {
    let ready = state
        .export
        .borrow()
        .as_ref()
        .is_some_and(ExportJob::is_finished);
    if !ready {
        return;
    }
    let job = state.export.borrow_mut().take().unwrap();
    let _ = KillTimer(Some(hwnd), EXPORT_TIMER);
    if let Err(error) = job.complete() {
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
