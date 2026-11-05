//! 宿主擦除提交：单任务轮询、版本/身份校验、原子历史提交及命中区域同步。
use super::{canvas::Canvas, State};
use windows::{
    core::{w, HSTRING},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};
pub(super) const TIMER: usize = 0x4F57;
pub(super) const MODE_COMMAND: usize = 0x1021;

pub(super) unsafe fn append_menu(
    menu: HMENU,
    state: &State,
    tool: crate::editor::Tool,
) -> windows::core::Result<()> {
    if state.erase_job.borrow().is_busy() {
        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, w!("擦除处理中…"))?;
    }
    if tool == crate::editor::Tool::Eraser {
        let checked = state.canvas.borrow().erase_annotations_only;
        AppendMenuW(
            menu,
            MF_STRING | if checked { MF_CHECKED } else { MF_UNCHECKED },
            MODE_COMMAND,
            w!("仅擦除栅格标注"),
        )?;
    }
    Ok(())
}
pub(super) fn toggle_mode(state: &State) {
    let mut canvas = state.canvas.borrow_mut();
    canvas.erase_annotations_only = !canvas.erase_annotations_only;
}

pub(super) unsafe fn submit(hwnd: HWND, state: &State) {
    let Some(request) = state.canvas.borrow_mut().erase_pending.take() else {
        return;
    };
    let plan = state.canvas.borrow().editor.prepare_raster_edit();
    let plan = match plan {
        Ok(Some(plan)) => plan,
        Ok(None) => return,
        Err(error) => {
            report(hwnd, &format!("{error:?}"));
            return;
        }
    };
    state.canvas.borrow_mut().erase_display = Some(request.clone());
    let started = state.erase_job.borrow_mut().start_edit(request, plan);
    if let Err(error) = started {
        state.canvas.borrow_mut().erase_display = None;
        report(hwnd, &format!("{error:?}"));
        return;
    }
    if SetTimer(Some(hwnd), TIMER, 30, None) == 0 {
        *state.erase_job.borrow_mut() = Default::default();
        state.canvas.borrow_mut().erase_display = None;
        report(hwnd, "Cannot schedule erase result");
    }
}
pub(super) fn accept(
    canvas: &mut Canvas,
    completed: crate::erase_job::Completed,
) -> Result<bool, String> {
    canvas.erase_display = None;
    if canvas.erase_gesture.is_some()
        || canvas.path_drag.is_some()
        || canvas.selection_drag.is_some()
        || canvas.marquee.is_some()
        || canvas.editor.revision() != completed.revision
        || !canvas
            .editor
            .layers()
            .is_some_and(|layers| layers.same_storage(&completed.source))
    {
        return Ok(false);
    }
    let result = completed.result.map_err(|error| format!("{error:?}"))?;
    let changed = if let Some(plan) = completed.refresh {
        canvas.editor.commit_raster_refresh(plan)
    } else {
        canvas
            .editor
            .replace_layers(completed.revision, &completed.source, result)
    }
    .map_err(|error| format!("{error:?}"))?;
    if changed {
        canvas.sync_raster_source();
        canvas.selected = None;
    }
    Ok(changed)
}
pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    let current = {
        let canvas = state.canvas.borrow();
        canvas.editor.layers().is_some_and(|layers| {
            state
                .erase_job
                .borrow()
                .is_current(layers, canvas.editor.revision())
        })
    };
    if !current {
        state.erase_job.borrow_mut().cancel();
    }
    let completed = state.erase_job.borrow_mut().poll();
    if !state.erase_job.borrow().is_busy() {
        let _ = KillTimer(Some(hwnd), TIMER);
        if state.canvas.borrow_mut().erase_display.take().is_some() {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
    if let Some(completed) = completed {
        let result = accept(&mut state.canvas.borrow_mut(), completed);
        match result {
            Ok(true) => {
                if let Err(error) = super::hit_region::update(hwnd, state) {
                    *state.error.borrow_mut() = Some(error);
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            Ok(false) => {}
            Err(error) => report(hwnd, &error),
        }
    }
}
unsafe fn report(hwnd: HWND, error: &str) {
    MessageBoxW(
        Some(hwnd),
        &HSTRING::from(format!("擦除未提交，原文档保留。\n{error}")),
        w!("Otter"),
        MB_OK | MB_ICONERROR,
    );
}

#[cfg(test)]
#[path = "erase_input_tests.rs"]
mod tests;
