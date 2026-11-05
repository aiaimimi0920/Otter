//! 整图翻转宿主：保留旧画面，完成后统一交换模型、栅格缓存及命中区域。
use super::{canvas::Canvas, State};
use crate::{
    editor::{EditError, RasterRefreshPlan},
    flip_job::Error,
    raster_flip::Axis,
};
use windows::{
    core::{w, Result, HSTRING},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};

pub(super) const HORIZONTAL: usize = 0x1030;
pub(super) const VERTICAL: usize = 0x1031;
pub(super) const TIMER: usize = 0x4F5A;

fn busy(state: &State) -> bool {
    state.crop_job.borrow().is_some()
        || state.flip_job.borrow().is_busy()
        || state.erase_job.borrow().is_busy()
        || state.resize_job.borrow().is_busy()
        || state.rasterize_job.borrow().is_some()
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let canvas = state.canvas.borrow();
    let display = state.resolved_display.borrow();
    let supported = !state.is_art();
    let enabled =
        !busy(state) && supported && (canvas.editor.layers().is_some() || display.is_some());
    let flags = MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED };
    AppendMenuW(menu, flags, HORIZONTAL, w!("水平翻转"))?;
    AppendMenuW(menu, flags, VERTICAL, w!("垂直翻转"))
}
pub(super) unsafe fn start(hwnd: HWND, state: &State, command: usize) {
    if busy(state) {
        return;
    }
    let axis = match command {
        HORIZONTAL => Axis::Horizontal,
        VERTICAL => Axis::Vertical,
        _ => return,
    };
    if !super::display_edit::prepare_content(state) {
        return;
    }
    let prepared = {
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        canvas.editor.prepare_flip(axis, |text| {
            super::annotation_text::measure_flip_metrics(text).map_err(|_| EditError::InvalidPoint)
        })
    };
    let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
    let _ = InvalidateRect(Some(hwnd), None, false);
    let plan = match prepared {
        Ok(Some(plan)) => plan,
        Ok(None) => return,
        Err(error) => {
            report(hwnd, &format!("{error:?}"));
            return;
        }
    };
    state.blur_job.borrow_mut().cancel();
    let started = state.flip_job.borrow_mut().start(plan);
    if let Err(error) = started {
        report(hwnd, &format!("{error:?}"));
        return;
    }
    // 定时器仅在任务活动期间存在；调度失败必须取消并回收任务。
    if SetTimer(Some(hwnd), TIMER, 30, None) == 0 {
        drop(std::mem::take(&mut *state.flip_job.borrow_mut()));
        report(hwnd, "Cannot schedule flip result");
    }
}
fn accept(canvas: &mut Canvas, plan: RasterRefreshPlan) -> std::result::Result<bool, EditError> {
    if canvas.selection_drag.is_some()
        || canvas.marquee.is_some()
        || canvas.path_drag.is_some()
        || canvas.erase_gesture.is_some()
    {
        return Ok(false);
    }
    if !canvas.editor.commit_raster_refresh(plan)? {
        return Ok(false);
    }
    canvas.selected = None;
    canvas.multi_selection = None;
    canvas.sync_raster_source();
    Ok(true)
}
pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    let ready = state.flip_job.borrow_mut().poll();
    let Some(ready) = ready else {
        return;
    };
    let _ = KillTimer(Some(hwnd), TIMER);
    match ready {
        Ok(plan) => {
            let result = accept(&mut state.canvas.borrow_mut(), plan);
            match result {
                Ok(true) => {
                    if let Err(error) = super::hit_region::update(hwnd, state) {
                        *state.error.borrow_mut() = Some(error);
                        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    }
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                Ok(false) => {}
                Err(error) => report(hwnd, &format!("{error:?}")),
            }
        }
        Err(Error::Cancelled) => {}
        Err(error) => report(hwnd, &format!("{error:?}")),
    }
}
unsafe fn report(hwnd: HWND, error: &str) {
    MessageBoxW(
        Some(hwnd),
        &HSTRING::from(format!("翻转未提交，原图与标注保留。\n{error}")),
        w!("Otter"),
        MB_OK | MB_ICONERROR,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editor::Tool, viewport::Point};
    fn setup() -> (Canvas, RasterRefreshPlan) {
        let mut canvas = Canvas::new(Some(super::super::image::Image {
            width: 2,
            height: 1,
            pixels: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
        }))
        .unwrap();
        canvas
            .editor
            .begin(Tool::Line, Point { x: 0.0, y: 0.0 })
            .unwrap();
        canvas.editor.extend(Point { x: 1.0, y: 1.0 }).unwrap();
        assert!(canvas.editor.commit());
        canvas.selected = Some((0, canvas.editor.revision()));
        let plan = canvas
            .editor
            .prepare_flip(Axis::Horizontal, |_| unreachable!())
            .unwrap()
            .unwrap()
            .compute(|| false)
            .unwrap();
        (canvas, plan)
    }
    #[test]
    fn commit_syncs_pixels_and_selection_and_undo_restores_source() {
        let (mut canvas, plan) = setup();
        let original = canvas.image.as_ref().unwrap().pixels.clone();
        assert!(accept(&mut canvas, plan).unwrap());
        assert!(canvas.selected.is_none());
        assert!(canvas.multi_selection.is_none());
        assert_eq!(
            &canvas.image.as_ref().unwrap().pixels[..],
            &[40, 50, 60, 255, 10, 20, 30, 255]
        );
        assert!(canvas.editor.undo());
        canvas.sync_raster_source();
        assert_eq!(&canvas.image.as_ref().unwrap().pixels[..], &original[..]);
    }
    #[test]
    fn active_drag_does_not_replace_image_or_selection() {
        let (mut canvas, plan) = setup();
        let revision = canvas.editor.revision();
        canvas.selection_drag = Some((Point { x: 0.0, y: 0.0 }, Point { x: 1.0, y: 0.0 }));
        assert!(!accept(&mut canvas, plan).unwrap());
        assert_eq!(canvas.editor.revision(), revision);
        assert_eq!(canvas.selected, Some((0, revision)));
        assert_eq!(canvas.image.as_ref().unwrap().pixels[0], 10);
    }
    #[test]
    fn replaced_document_rejects_finished_flip() {
        let (mut canvas, plan) = setup();
        canvas.editor.undo();
        assert!(!accept(&mut canvas, plan).unwrap());
        assert_eq!(canvas.image.as_ref().unwrap().pixels[0], 10);
        assert!(canvas.editor.redo());
    }
}
