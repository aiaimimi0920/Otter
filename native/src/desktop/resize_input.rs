//! 含模糊的缩放在后台准备整组效果，UI 保留旧帧；只在任务活动期间轮询。
use super::{canvas::Canvas, State};
use crate::{
    editor::{ResizePlan, Tool},
    resize_job::Error,
    viewport::Point,
};
use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        Graphics::Gdi::InvalidateRect,
        UI::WindowsAndMessaging::*,
    },
};
pub(super) const TIMER: usize = 0x4F59;

pub(super) unsafe fn submit_pending(hwnd: HWND, state: &State) {
    let plan = state.canvas.borrow_mut().scale_pending.take();
    let Some(plan) = plan else { return };
    if state.crop_job.borrow().is_some()
        || state.flip_job.borrow().is_busy()
        || state.resize_job.borrow().is_busy()
        || state.erase_job.borrow().is_busy()
        || state.rasterize_job.borrow().is_some()
    {
        state.canvas.borrow_mut().clear_scale_display();
        return;
    }
    if !plan.strokes().any(|(_, stroke)| stroke.tool == Tool::Blur) {
        let result = accept(&mut state.canvas.borrow_mut(), plan);
        match result {
            Ok(true) => refresh(hwnd, state),
            Ok(false) => {}
            Err(error) => report(hwnd, &format!("{error:?}")),
        }
        return;
    }
    let result = state.resize_job.borrow_mut().start(plan);
    if let Err(error) = result {
        state.canvas.borrow_mut().clear_scale_display();
        report(hwnd, &format!("{error:?}"));
        return;
    }
    if SetTimer(Some(hwnd), TIMER, 30, None) == 0 {
        let job = std::mem::take(&mut *state.resize_job.borrow_mut());
        drop(job);
        state.canvas.borrow_mut().clear_scale_display();
        report(hwnd, "Cannot schedule resize result");
    }
}

pub(super) unsafe fn uniform(hwnd: HWND, state: &State, factor: f32) {
    start(
        hwnd,
        state,
        Point {
            x: factor,
            y: factor,
        },
    );
}

pub(super) unsafe fn axis_step(hwnd: HWND, state: &State, horizontal: bool, enlarge: bool) {
    let factor = if enlarge { 1.1 } else { 1.0 / 1.1 };
    let factors = if horizontal {
        Point { x: factor, y: 1.0 }
    } else {
        Point { x: 1.0, y: factor }
    };
    start(hwnd, state, factors);
}

pub(super) unsafe fn start(hwnd: HWND, state: &State, factors: Point) {
    if state.crop_job.borrow().is_some()
        || state.flip_job.borrow().is_busy()
        || state.resize_job.borrow().is_busy()
        || state.erase_job.borrow().is_busy()
        || state.rasterize_job.borrow().is_some()
    {
        return;
    }
    let prepared = {
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        let indices = canvas.selected_indices();
        if !indices
            .iter()
            .any(|index| canvas.editor.elements()[*index].tool == Tool::Blur)
        {
            canvas.resize_selected_axes(factors);
            None
        } else {
            crate::group_bounds::center(
                indices
                    .iter()
                    .map(|index| &canvas.editor.elements()[*index]),
            )
            .map(|pivot| {
                canvas.editor.prepare_resize_many_mode(
                    &indices,
                    pivot,
                    factors,
                    canvas.transform_own,
                    |text| {
                        super::annotation_text::measure(text)
                            .map_err(|_| crate::editor::EditError::InvalidPoint)
                    },
                )
            })
        }
    };
    match prepared {
        Some(Ok(Some(plan))) => {
            let result = state.resize_job.borrow_mut().start(plan);
            if let Err(error) = result {
                report(hwnd, &format!("{error:?}"));
                return;
            }
            if SetTimer(Some(hwnd), TIMER, 30, None) == 0 {
                let job = std::mem::take(&mut *state.resize_job.borrow_mut());
                drop(job);
                report(hwnd, "Cannot schedule resize result");
            }
        }
        Some(Err(error)) => report(hwnd, &format!("{error:?}")),
        _ => refresh(hwnd, state),
    }
}

fn accept(canvas: &mut Canvas, plan: ResizePlan) -> Result<bool, crate::editor::EditError> {
    let indices: Vec<_> = plan.strokes().map(|(index, _)| index).collect();
    if indices != canvas.selected_indices()
        || canvas.selection_drag.is_some()
        || canvas.marquee.is_some()
        || canvas.path_drag.is_some()
        || canvas.erase_gesture.is_some()
    {
        return Ok(false);
    }
    let changed = canvas.editor.commit_resize(plan)?;
    if changed {
        let revision = canvas.editor.revision();
        canvas.selected = indices.first().map(|index| (*index, revision));
        canvas.multi_selection = (indices.len() > 1).then_some((indices, revision));
    }
    Ok(changed)
}

pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    let result = state.resize_job.borrow_mut().poll();
    let Some(result) = result else { return };
    let _ = KillTimer(Some(hwnd), TIMER);
    if state.canvas.borrow_mut().clear_scale_display() {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    match result {
        Ok(plan) => {
            let accepted = accept(&mut state.canvas.borrow_mut(), plan);
            match accepted {
                Ok(true) => refresh(hwnd, state),
                Ok(false) => {}
                Err(error) => report(hwnd, &format!("{error:?}")),
            }
        }
        Err(Error::Cancelled) => {}
        Err(error) => report(hwnd, &format!("{error:?}")),
    }
}
unsafe fn refresh(hwnd: HWND, state: &State) {
    if let Err(error) = super::hit_region::update(hwnd, state) {
        *state.error.borrow_mut() = Some(error);
        let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
    let _ = InvalidateRect(Some(hwnd), None, false);
}
unsafe fn report(hwnd: HWND, error: &str) {
    MessageBoxW(
        Some(hwnd),
        &HSTRING::from(format!("缩放未提交，原标注保留。\n{error}")),
        w!("Otter"),
        MB_OK | MB_ICONERROR,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Canvas, ResizePlan) {
        let mut canvas = Canvas::new(Some(super::super::image::Image {
            width: 16,
            height: 16,
            pixels: vec![255; 16 * 16 * 4].into(),
        }))
        .unwrap();
        let pixels = canvas.image.as_ref().unwrap().pixels.clone();
        canvas
            .editor
            .begin_blur(
                Point { x: 4.0, y: 8.0 },
                crate::blur_image::BlurImage::new(16, 16, pixels, 4).unwrap(),
            )
            .unwrap();
        assert!(canvas.editor.commit());
        canvas
            .editor
            .begin(Tool::Brush, Point { x: 12.0, y: 8.0 })
            .unwrap();
        assert!(canvas.editor.commit());
        canvas.select_tool(Tool::Select);
        let revision = canvas.editor.revision();
        canvas.selected = Some((0, revision));
        canvas.multi_selection = Some((vec![0, 1], revision));
        let plan = canvas
            .editor
            .prepare_resize_many(
                &[0, 1],
                Point { x: 8.0, y: 8.0 },
                Point { x: 2.0, y: 2.0 },
                |_| unreachable!(),
            )
            .unwrap()
            .unwrap();
        (canvas, plan)
    }
    fn finish(plan: ResizePlan) -> ResizePlan {
        let mut job = crate::resize_job::Job::default();
        job.start(plan).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(result) = job.poll() {
                return result.unwrap();
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    #[test]
    fn worker_handoff_updates_whole_selection_and_undoes_effect_with_geometry() {
        let (mut canvas, plan) = setup();
        let revision = canvas.editor.revision();
        assert!(accept(&mut canvas, finish(plan)).unwrap());
        assert_eq!(canvas.editor.revision(), revision + 1);
        assert_eq!(canvas.selected_indices(), vec![0, 1]);
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            8
        );
        assert_eq!(canvas.editor.elements()[1].points[0].x, 16.0);
        assert!(canvas.editor.undo());
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            4
        );
        assert_eq!(canvas.editor.elements()[1].points[0].x, 12.0);
    }
    #[test]
    fn changed_selection_or_document_rejects_completed_candidate() {
        let (mut canvas, plan) = setup();
        let revision = canvas.editor.revision();
        canvas.multi_selection = None;
        assert!(!accept(&mut canvas, finish(plan)).unwrap());
        assert_eq!(canvas.editor.revision(), revision);
        let (mut canvas, plan) = setup();
        assert!(canvas.editor.undo());
        assert!(!accept(&mut canvas, finish(plan)).unwrap());
    }
}
