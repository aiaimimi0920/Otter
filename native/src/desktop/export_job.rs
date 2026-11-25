//! Allow one export task at a time; share pixels through Arc and enable completion checks only while tasks exist.
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
        // Do not leak unfinished writes into unowned background tasks; wait for this export on exit.
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
    let filter: Vec<u16> = format!("{}\0*.png\0\0", crate::ui_text::text("file-filter-png"))
        .encode_utf16()
        .collect();
    let mut options = OPENFILENAMEW {
        lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
        hwndOwner: hwnd,
        lpstrFile: windows::core::PWSTR(name.as_mut_ptr()),
        nMaxFile: name.len() as u32,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrDefExt: w!("png"),
        Flags: OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };
    if !GetSaveFileNameW(&mut options).as_bool() {
        if CommDlgExtendedError() != COMMON_DLG_ERRORS(0) {
            report(hwnd, crate::ui_text::text("export-dialog-failed"));
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
        report(hwnd, crate::ui_text::text("export-document-changed"));
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
        report(hwnd, crate::ui_text::text("export-schedule-failed"));
        return;
    }
    match ExportJob::start(operation) {
        Ok(job) => *state.export.borrow_mut() = Some(job),
        Err(error) => {
            let _ = KillTimer(Some(hwnd), EXPORT_TIMER);
            report(
                hwnd,
                &crate::ui_text::format(
                    "export-start-failed",
                    &[("error", error.to_string().into())],
                ),
            );
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
        &HSTRING::from(crate::ui_text::text("export-title")),
        MB_OK | MB_ICONERROR,
    );
}
