//! 裁剪与模糊的单任务交付；活动期间保留旧视图，完成后同步窗口，不产生中间历史。
use super::{canvas::Canvas, State};
use crate::{blur::BlurError, crop_geometry::Rect, editor::RasterRefreshPlan};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
};
use windows::{
    core::{w, HSTRING},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};
pub(super) const TIMER: usize = 0x4F5B;
pub(super) struct Job {
    cancel: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<RasterRefreshPlan, BlurError>>>,
}
impl Job {
    fn start(plan: RasterRefreshPlan) -> std::io::Result<Self> {
        let cancel = Arc::new(AtomicBool::new(false));
        let signal = cancel.clone();
        let worker = thread::Builder::new()
            .name("otter-crop".into())
            .spawn(move || plan.compute(|| signal.load(Ordering::Acquire)))?;
        Ok(Self {
            cancel,
            worker: Some(worker),
        })
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
pub(super) fn prepare(canvas: &mut Canvas, selection: Option<Rect>) {
    let result = canvas.editor.prepare_crop_refresh(selection);
    match result {
        Ok(Some(plan)) if plan.requires_effect_compute() => canvas.crop_pending = Some(plan),
        Ok(Some(plan)) => match plan.compute(|| false) {
            Ok(plan) => {
                if let Err(error) = accept(canvas, plan) {
                    eprintln!("native-crop: {error:?}");
                }
            }
            Err(error) => eprintln!("native-crop: {error:?}"),
        },
        Ok(None) => {}
        Err(error) => eprintln!("native-crop: {error:?}; document unchanged"),
    }
}
fn accept(canvas: &mut Canvas, plan: RasterRefreshPlan) -> Result<bool, crate::editor::EditError> {
    if canvas.selection_drag.is_some()
        || canvas.marquee.is_some()
        || canvas.path_drag.is_some()
        || canvas.erase_gesture.is_some()
    {
        return Ok(false);
    }
    let changed = canvas.editor.commit_raster_refresh(plan)?;
    if changed {
        canvas.selected = None;
        canvas.multi_selection = None;
        canvas.sync_raster_source();
    }
    Ok(changed)
}
pub(super) fn cancel(state: &State) {
    if let Some(job) = state.crop_job.borrow().as_ref() {
        job.cancel.store(true, Ordering::Release);
    }
}
pub(super) unsafe fn submit(hwnd: HWND, state: &State) {
    let Some(plan) = state.canvas.borrow_mut().crop_pending.take() else {
        return;
    };
    if state.crop_job.borrow().is_some()
        || state.erase_job.borrow().is_busy()
        || state.resize_job.borrow().is_busy()
        || state.flip_job.borrow().is_busy()
        || state.rasterize_job.borrow().is_some()
    {
        return;
    }
    state.blur_job.borrow_mut().cancel();
    match Job::start(plan) {
        Ok(job) => {
            *state.crop_job.borrow_mut() = Some(job);
            if SetTimer(Some(hwnd), TIMER, 30, None) == 0 {
                drop(state.crop_job.borrow_mut().take());
                report(hwnd, "Cannot schedule crop result");
            }
        }
        Err(error) => report(hwnd, &error.to_string()),
    }
}
pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    if !state.crop_job.borrow().as_ref().is_some_and(|job| {
        job.worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
    }) {
        return;
    }
    let mut job = state.crop_job.borrow_mut().take().unwrap();
    let result = job.worker.take().unwrap().join();
    let _ = KillTimer(Some(hwnd), TIMER);
    if job.cancel.load(Ordering::Acquire) {
        return;
    }
    match result {
        Ok(Ok(plan)) => {
            let accepted = accept(&mut state.canvas.borrow_mut(), plan);
            match accepted {
                Ok(true) => {
                    if let Err(error) = super::crop_window::sync(hwnd, state)
                        .and_then(|_| super::hit_region::update(hwnd, state))
                    {
                        *state.error.borrow_mut() = Some(error);
                        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                Ok(false) => {}
                Err(error) => report(hwnd, &format!("{error:?}")),
            }
        }
        Ok(Err(BlurError::Cancelled)) => {}
        Ok(Err(error)) => report(hwnd, &format!("{error:?}")),
        Err(_) => report(hwnd, "Crop worker failed"),
    }
}
unsafe fn report(hwnd: HWND, error: &str) {
    MessageBoxW(
        Some(hwnd),
        &HSTRING::from(format!("裁剪未提交，原画框及效果保留。\n{error}")),
        w!("Otter"),
        MB_OK | MB_ICONERROR,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editor::Tool, viewport::Point};
    fn canvas() -> Canvas {
        let mut canvas = Canvas::new(Some(crate::desktop::image::Image {
            width: 16,
            height: 16,
            pixels: vec![0; 1024].into(),
        }))
        .unwrap();
        canvas.editor = crate::editor::Editor::with_layers(crate::erase_layers::Layers {
            annotation_size: None,
            size: (16, 16),
            base: vec![0; 1024].into(),
            annotations: Some([0, 0, 128, 128].repeat(256).into()),
        })
        .unwrap();
        canvas
            .editor
            .begin_blur(
                Point { x: 10.0, y: 6.0 },
                crate::blur_image::BlurImage::with_strength(16, 16, vec![0; 1024].into(), 2.4)
                    .unwrap(),
            )
            .unwrap();
        canvas.editor.commit();
        canvas.select_tool(Tool::Select);
        canvas.viewport = crate::viewport::Viewport {
            origin: Point::default(),
            scale: 1.0,
        };
        canvas.crop_mode = true;
        canvas
    }
    fn finish(mut job: Job) -> RasterRefreshPlan {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !job.worker.as_ref().unwrap().is_finished() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        job.worker.take().unwrap().join().unwrap().unwrap()
    }
    #[test]
    fn pointer_crop_and_reset_wait_for_worker_then_commit_view_and_effect() {
        let mut canvas = canvas();
        let old = canvas.editor.elements()[0].blur.as_ref().unwrap().clone();
        let revision = canvas.editor.revision();
        assert!(canvas.pointer_down(Point { x: 8.0, y: 4.0 }));
        assert!(canvas.pointer_up(Point { x: 16.0, y: 12.0 }));
        assert!(canvas.editor.crop_view().is_none());
        assert_eq!(canvas.editor.revision(), revision);
        let job = Job::start(canvas.crop_pending.take().unwrap()).unwrap();
        assert!(accept(&mut canvas, finish(job)).unwrap());
        let crop = canvas.editor.crop_view();
        assert!(crop.is_some());
        assert_eq!(canvas.editor.revision(), revision + 1);
        let cropped = canvas.editor.elements()[0].blur.as_ref().unwrap().clone();
        assert!(!cropped.pixels().same_storage(old.pixels()));
        prepare(&mut canvas, None);
        assert_eq!(canvas.editor.crop_view(), crop);
        let job = Job::start(canvas.crop_pending.take().unwrap()).unwrap();
        assert!(accept(&mut canvas, finish(job)).unwrap());
        assert!(canvas.editor.crop_view().is_none());
        assert!(canvas.editor.undo());
        assert_eq!(canvas.editor.crop_view(), crop);
        assert!(canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(cropped.pixels()));
        assert!(canvas.editor.undo());
        assert!(canvas.editor.crop_view().is_none());
        assert!(canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(old.pixels()));
    }
    #[test]
    fn stale_crop_worker_and_dropped_worker_do_not_change_document() {
        let mut canvas = canvas();
        let rect = Rect {
            x: 2.0,
            y: 2.0,
            width: 8.0,
            height: 8.0,
        };
        prepare(&mut canvas, Some(rect));
        let job = Job::start(canvas.crop_pending.take().unwrap()).unwrap();
        assert!(canvas.editor.undo());
        assert!(!accept(&mut canvas, finish(job)).unwrap());
        assert!(canvas.editor.crop_view().is_none());
        assert!(canvas.editor.redo());
        prepare(&mut canvas, Some(rect));
        let revision = canvas.editor.revision();
        drop(Job::start(canvas.crop_pending.take().unwrap()).unwrap());
        assert_eq!(canvas.editor.revision(), revision);
        assert!(canvas.editor.crop_view().is_none());
    }
}
