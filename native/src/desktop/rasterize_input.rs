//! 单后台栅格化事务；COM 只在工作线程内存在，回执通过版本和源层身份校验后提交。
use super::{canvas::Canvas, State};
use crate::{
    editor::{Editor, RasterRefreshPlan},
    erase_layers::Layers,
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
use windows::{
    core::{w, HSTRING},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};
pub(super) const TIMER: usize = 0x4F58;
pub(super) const SELECTED: usize = 0x1022;
pub(super) const ALL: usize = 0x1023;

struct Request {
    source: Layers,
    revision: u64,
    indices: Vec<usize>,
    refresh: Option<RasterRefreshPlan>,
}
pub(super) struct Job {
    request: Request,
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<RasterRefreshPlan, String>>>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        // WIC/D2D 同步调用不可中断；关闭时等待当前调用结束，不遗留线程或 COM 对象。
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
fn prepare(canvas: &Canvas, selected: bool) -> Option<(Request, Editor)> {
    let indices = if selected {
        canvas.selected_indices()
    } else {
        (0..canvas.editor.elements().len()).collect()
    };
    let snapshot = canvas.editor.rasterize_snapshot(&indices).ok()?;
    let refresh = canvas.editor.prepare_rasterize_edit(&indices).ok()??;
    Some((
        Request {
            source: canvas.editor.layers()?.clone(),
            revision: canvas.editor.revision(),
            indices,
            refresh: Some(refresh),
        },
        snapshot,
    ))
}
fn spawn(mut request: Request, snapshot: Editor) -> Result<Job, String> {
    if request.indices.is_empty() {
        return Err("Empty rasterization".into());
    }
    let refresh = request
        .refresh
        .take()
        .ok_or("Missing raster refresh plan")?;
    let source = request.source.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = cancel.clone();
    let worker = std::thread::Builder::new()
        .name("otter-rasterize".into())
        .spawn(move || {
            if signal.load(Ordering::Acquire) {
                return Err("Rasterization cancelled".into());
            }
            let result =
                super::rasterize::render_snapshot(&source, &snapshot).map_err(|e| e.to_string());
            if signal.load(Ordering::Acquire) {
                Err("Rasterization cancelled".into())
            } else {
                let layers = result?;
                refresh
                    .with_candidate(layers)
                    .and_then(|plan| plan.compute(|| signal.load(Ordering::Acquire)))
                    .map_err(|error| format!("{error:?}"))
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(Job {
        request,
        cancel,
        worker: Some(worker),
    })
}
pub(super) unsafe fn append_menu(menu: HMENU, state: &State) -> windows::core::Result<()> {
    let busy = state.rasterize_job.borrow().is_some() || state.erase_job.borrow().is_busy();
    let (selected, all) = {
        let canvas = state.canvas.borrow();
        let display = state.resolved_display.borrow();
        let supported = !state.is_art();
        (
            supported && display.is_none() && !canvas.selected_indices().is_empty(),
            supported
                && display.as_ref().map_or_else(
                    || !canvas.editor.elements().is_empty(),
                    |frame| {
                        frame
                            .editor
                            .as_deref()
                            .is_some_and(|editor| !editor.elements().is_empty())
                    },
                ),
        )
    };
    AppendMenuW(
        menu,
        MF_STRING
            | if selected && !busy {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        SELECTED,
        w!("栅格化选中标注"),
    )?;
    AppendMenuW(
        menu,
        MF_STRING | if all && !busy { MF_ENABLED } else { MF_GRAYED },
        ALL,
        w!("栅格化全部标注"),
    )?;
    if state.rasterize_job.borrow().is_some() {
        AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, w!("栅格化处理中…"))?;
    }
    Ok(())
}
pub(super) unsafe fn start(hwnd: HWND, state: &State, selected: bool) {
    if state.rasterize_job.borrow().is_some()
        || state.erase_job.borrow().is_busy()
        || state.resize_job.borrow().is_busy()
        || state.flip_job.borrow().is_busy()
        || state.crop_job.borrow().is_some()
    {
        return;
    }
    if selected && state.resolved_display.borrow().is_some() {
        return;
    }
    if !super::display_edit::prepare_content(state) {
        return;
    }
    state.canvas.borrow_mut().cancel_gesture();
    let request = prepare(&state.canvas.borrow(), selected);
    let Some((request, snapshot)) = request else {
        return;
    };
    match spawn(request, snapshot) {
        Ok(job) => {
            *state.rasterize_job.borrow_mut() = Some(job);
            if SetTimer(Some(hwnd), TIMER, 30, None) == 0 {
                let job = state.rasterize_job.borrow_mut().take();
                drop(job);
                report(hwnd, "Cannot schedule rasterization result");
            }
        }
        Err(error) => report(hwnd, &error),
    }
}
pub(super) fn cancel(state: &State) {
    if let Some(job) = state.rasterize_job.borrow().as_ref() {
        job.cancel.store(true, Ordering::Release);
    }
}
fn accept(
    canvas: &mut Canvas,
    request: &Request,
    result: Result<RasterRefreshPlan, String>,
) -> Result<bool, String> {
    if canvas.editor.revision() != request.revision
        || canvas.editor.draft().is_some()
        || canvas.erase_gesture.is_some()
        || canvas.path_drag.is_some()
        || canvas.selection_drag.is_some()
        || canvas.marquee.is_some()
        || !canvas
            .editor
            .layers()
            .is_some_and(|layers| layers.same_storage(&request.source))
    {
        return Ok(false);
    }
    let changed = canvas
        .editor
        .commit_raster_refresh(result?)
        .map_err(|e| format!("{e:?}"))?;
    if changed {
        canvas.selected = None;
        canvas.sync_raster_source();
    }
    Ok(changed)
}
pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    let ready = state.rasterize_job.borrow().as_ref().is_some_and(|job| {
        job.worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
    });
    if !ready {
        return;
    }
    let mut job = state.rasterize_job.borrow_mut().take().unwrap();
    let _ = KillTimer(Some(hwnd), TIMER);
    let result = job
        .worker
        .take()
        .unwrap()
        .join()
        .unwrap_or_else(|_| Err("Rasterization worker failed".into()));
    if job.cancel.load(Ordering::Acquire) {
        return;
    }
    let accepted = accept(&mut state.canvas.borrow_mut(), &job.request, result);
    match accepted {
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
unsafe fn report(hwnd: HWND, error: &str) {
    MessageBoxW(
        Some(hwnd),
        &HSTRING::from(format!("栅格化未提交，原标注保留。\n{error}")),
        w!("Otter"),
        MB_OK | MB_ICONERROR,
    );
}

#[cfg(test)]
#[path = "rasterize_input_tests.rs"]
mod tests;
