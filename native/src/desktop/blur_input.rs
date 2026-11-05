//! 模糊准备只在显式选工具时启动；结果入画布前核验源图、尺寸、版本和工具。
use super::blur_selection::increment as selection_request;
use super::{canvas::Canvas, State};
#[cfg(test)]
#[path = "blur_frame_tests.rs"]
mod frame_tests;
use crate::{
    blur::Region,
    blur_image::BlurImage,
    blur_job::{Completed, Request},
    editor::Tool,
};
use windows::{
    core::{w, HSTRING},
    Win32::{Foundation::*, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*},
};
pub(super) const TIMER: usize = 0x4F56;

#[cfg(test)]
#[path = "blur_precision_tests.rs"]
mod precision_tests;

pub(super) fn selected_strength(canvas: &Canvas) -> Option<f32> {
    if canvas.tool != Tool::Select {
        return None;
    }
    canvas.selected_index().and_then(|index| {
        canvas.editor.elements()[index]
            .blur
            .as_ref()
            .map(BlurImage::strength)
    })
}
fn target_matches(canvas: &Canvas, request: &Request) -> bool {
    match request.target {
        Some(index) => {
            canvas.tool == Tool::Select
                && canvas.selected_index() == Some(index)
                && canvas.editor.elements()[index].tool == Tool::Blur
        }
        None => canvas.tool == Tool::Blur && request.sigma == canvas.tool_profiles.blur_strength(),
    }
}
pub(super) unsafe fn adjust_strength(hwnd: HWND, state: &State, increment: f32) {
    if !increment.is_finite() {
        return;
    }
    let selected = selected_strength(&state.canvas.borrow()).is_some();
    if selected {
        let previous = state.blur_job.borrow().current_request().cloned();
        let request = {
            let mut canvas = state.canvas.borrow_mut();
            canvas.cancel_gesture();
            selection_request(&canvas, previous.as_ref(), increment)
        };
        if let Some(request) = request {
            submit(hwnd, state, request);
        }
        return;
    }
    let changed = {
        let mut canvas = state.canvas.borrow_mut();
        let old = canvas.tool_profiles.blur_strength();
        let next = (old + increment).clamp(2.0, 64.0);
        if next == old {
            false
        } else {
            canvas.cancel_gesture();
            canvas.tool_profiles.set_blur_strength(next);
            canvas.blur_ready = None;
            true
        }
    };
    if changed {
        start(hwnd, state);
    }
}

pub(super) unsafe fn start(hwnd: HWND, state: &State) {
    let request = {
        let mut canvas = state.canvas.borrow_mut();
        canvas.sync_raster_source();
        if canvas.image.is_none() {
            return;
        }
        canvas.cancel_gesture();
        canvas.select_tool(Tool::Blur);
        let image = canvas.image.as_ref().unwrap();
        if canvas
            .blur_ready
            .as_ref()
            .is_some_and(|(source, ready, frame, base_projection)| {
                canvas
                    .editor
                    .layers()
                    .is_some_and(|layers| source.same_storage(layers))
                    && ready.strength() == canvas.tool_profiles.blur_strength()
                    && *frame == canvas.editor.annotation_frame()
                    && *base_projection == canvas.editor.raster_base_projection()
            })
        {
            drop(canvas);
            let _ = super::hit_region::update(hwnd, state);
            let _ = InvalidateRect(Some(hwnd), None, false);
            return;
        }
        Request {
            stamp: canvas.editor.document_stamp(),
            target: None,
            annotation_frame: canvas.editor.annotation_frame(),
            base_projection: canvas.editor.raster_base_projection(),
            source: canvas.editor.layers().unwrap().clone(),
            size: (image.width, image.height),
            area: Region {
                x: 0,
                y: 0,
                width: image.width,
                height: image.height,
            },
            sigma: canvas.tool_profiles.blur_strength(),
            revision: canvas.editor.revision(),
        }
    };
    submit(hwnd, state, request);
}
pub(super) unsafe fn submit(hwnd: HWND, state: &State, request: Request) {
    let result = state.blur_job.borrow_mut().request(request);
    if let Err(error) = result {
        report(hwnd, &format!("{error:?}"));
        return;
    }
    if SetTimer(Some(hwnd), TIMER, 50, None) == 0 {
        // 无轮询器不能留下无人消费的任务；取消并回收后再报告。
        *state.blur_job.borrow_mut() = Default::default();
        report(hwnd, "无法启动模糊任务定时器。");
    }
    let _ = super::hit_region::update(hwnd, state);
    let _ = InvalidateRect(Some(hwnd), None, false);
}

fn accept(canvas: &mut Canvas, completed: Completed) -> Result<bool, String> {
    let request = completed.request;
    let Some(image) = &canvas.image else {
        return Ok(false);
    };
    if !target_matches(canvas, &request)
        || !request.stamp.matches(&canvas.editor)
        || canvas.editor.revision() != request.revision
        || request.annotation_frame != canvas.editor.annotation_frame()
        || request.base_projection != canvas.editor.raster_base_projection()
        || canvas.editor.draft().is_some()
        || canvas.path_drag.is_some()
        || canvas.selection_drag.is_some()
        || canvas.marquee.is_some()
        || !canvas
            .editor
            .layers()
            .is_some_and(|layers| request.source.same_storage(layers))
        || request.size != (image.width, image.height)
        || request.area.x != 0
        || request.area.y != 0
        || request.area.width != image.width
        || request.area.height != image.height
    {
        return Ok(false);
    }
    let pixels = completed.result.map_err(|error| format!("{error:?}"))?;
    let prepared = BlurImage::with_strength(image.width, image.height, pixels, request.sigma)
        .ok_or("模糊图像尺寸不匹配")?;
    if let Some(index) = request.target {
        let changed = canvas
            .editor
            .replace_blur(index, prepared)
            .map_err(|e| format!("{e:?}"))?;
        if changed {
            canvas.selected = Some((index, canvas.editor.revision()));
        }
        Ok(changed)
    } else {
        canvas.blur_ready = Some((
            request.source,
            prepared,
            request.annotation_frame,
            request.base_projection,
        ));
        Ok(true)
    }
}

pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    let current = {
        let canvas = state.canvas.borrow();
        state
            .blur_job
            .borrow()
            .current_request()
            .is_some_and(|r| r.stamp.matches(&canvas.editor) && target_matches(&canvas, r))
            && canvas.editor.layers().is_some_and(|layers| {
                state
                    .blur_job
                    .borrow()
                    .is_current(layers, canvas.editor.revision())
            })
    };
    if !current {
        state.blur_job.borrow_mut().cancel();
    }
    let completed = state.blur_job.borrow_mut().poll();
    if !state.blur_job.borrow().is_busy() {
        let _ = KillTimer(Some(hwnd), TIMER);
    }
    if let Some(completed) = completed {
        let result = accept(&mut state.canvas.borrow_mut(), completed);
        match result {
            Ok(true) => {
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
        &HSTRING::from(format!("模糊准备失败，原图和标注未修改。\n{error}")),
        w!("Otter"),
        MB_OK | MB_ICONERROR,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    fn canvas() -> Canvas {
        let mut canvas = Canvas::new(Some(super::super::image::Image {
            width: 16,
            height: 16,
            pixels: vec![255; 16 * 16 * 4].into(),
        }))
        .unwrap();
        canvas.select_tool(Tool::Blur);
        canvas
    }
    fn completed(canvas: &Canvas) -> Completed {
        Completed {
            request: Request {
                stamp: canvas.editor.document_stamp(),
                target: None,
                annotation_frame: canvas.editor.annotation_frame(),
                base_projection: canvas.editor.raster_base_projection(),
                source: canvas.editor.layers().unwrap().clone(),
                size: (16, 16),
                area: Region {
                    x: 0,
                    y: 0,
                    width: 16,
                    height: 16,
                },
                sigma: canvas.tool_profiles.blur_strength(),
                revision: canvas.editor.revision(),
            },
            result: Ok(vec![128; 16 * 16 * 4].into()),
        }
    }
    #[test]
    fn accepted_result_enables_input_without_changing_history_and_replace_clears_it() {
        let mut canvas = canvas();
        let point = crate::viewport::Point { x: 25.0, y: 25.0 };
        assert!(!canvas.pointer_down(point));
        let revision = canvas.editor.revision();
        let result = completed(&canvas);
        assert!(accept(&mut canvas, result).unwrap());
        assert_eq!(canvas.editor.revision(), revision);
        assert!(canvas.pointer_down(point));
        canvas.pointer_up(point);
        assert_eq!(canvas.editor.elements()[0].tool, Tool::Blur);
        canvas
            .replace_image(super::super::image::Image {
                width: 16,
                height: 16,
                pixels: vec![0; 16 * 16 * 4].into(),
            })
            .unwrap();
        assert!(canvas.blur_ready.is_none());
    }
    #[test]
    fn strength_change_rejects_old_result_and_keeps_existing_strokes_unchanged() {
        let mut canvas = canvas();
        let mut stale = completed(&canvas);
        let first = completed(&canvas);
        assert!(accept(&mut canvas, first).unwrap());
        let point = crate::viewport::Point { x: 25.0, y: 25.0 };
        assert!(canvas.pointer_down(point));
        canvas.pointer_up(point);
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            8
        );
        assert!(!canvas.tool_profiles.set_blur_strength(1.0));
        assert!(!canvas.tool_profiles.set_blur_strength(65.0));
        assert!(canvas.tool_profiles.set_blur_strength(16.0));
        let revision = canvas.editor.revision();
        stale.request.revision = revision; // 只让强度不同，独立验证强度准入。
        assert!(!canvas.pointer_down(point)); // 旧准备快照不能被当作新强度使用。
        assert!(!accept(&mut canvas, stale).unwrap());
        let current = completed(&canvas);
        assert!(accept(&mut canvas, current).unwrap());
        assert_eq!(canvas.editor.revision(), revision);
        assert!(canvas.pointer_down(point));
        canvas.pointer_up(point);
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            8
        );
        assert_eq!(
            canvas.editor.elements()[1].blur.as_ref().unwrap().sigma(),
            16
        );
        canvas.editor.undo();
        canvas.editor.redo();
        assert_eq!(
            canvas.editor.elements()[1].blur.as_ref().unwrap().sigma(),
            16
        );
        canvas.select_tool(Tool::Brush);
        canvas.select_tool(Tool::Blur);
        assert_eq!(canvas.tool_profiles.blur_strength(), 16.0);
    }

    #[test]
    fn selected_strength_recompute_is_atomic_and_preserves_geometry_and_defaults() {
        let mut canvas = canvas();
        let ready = completed(&canvas);
        accept(&mut canvas, ready).unwrap();
        let point = crate::viewport::Point { x: 25.0, y: 25.0 };
        canvas.pointer_down(point);
        canvas.pointer_up(point);
        canvas.select_tool(Tool::Select);
        canvas.selected = Some((0, canvas.editor.revision()));
        let before = canvas.editor.elements()[0].clone();
        let first = selection_request(&canvas, None, 1.0).unwrap();
        let latest = selection_request(&canvas, Some(&first), 1.0).unwrap();
        assert_eq!(latest.sigma, 10.0);
        assert_eq!(latest.target, Some(0));
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            8
        );
        let mut job = crate::blur_job::Job::default();
        job.request(latest).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let result = loop {
            if let Some(result) = job.poll() {
                break result;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(1));
        };
        assert!(accept(&mut canvas, result).unwrap());
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            10
        );
        assert_eq!(canvas.editor.elements()[0].points, before.points);
        assert_eq!(canvas.editor.elements()[0].style(), before.style());
        assert_eq!(canvas.tool_profiles.blur_strength(), 8.0);
        assert_eq!(canvas.selected_index(), Some(0));
        canvas.editor.undo();
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            8
        );
        assert!(canvas.editor.elements()[0]
            .blur
            .as_ref()
            .unwrap()
            .pixels()
            .same_storage(before.blur.as_ref().unwrap().pixels()));
        canvas.editor.redo();
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            10
        );
    }

    #[test]
    fn selected_recompute_rejects_changed_selection() {
        let mut canvas = canvas();
        let ready = completed(&canvas);
        accept(&mut canvas, ready).unwrap();
        let point = crate::viewport::Point { x: 25.0, y: 25.0 };
        canvas.pointer_down(point);
        canvas.pointer_up(point);
        canvas.select_tool(Tool::Select);
        canvas.selected = Some((0, canvas.editor.revision()));
        let request = selection_request(&canvas, None, 1.0).unwrap();
        let revision = canvas.editor.revision();
        canvas.selected = None;
        assert!(!accept(
            &mut canvas,
            Completed {
                request,
                result: Ok(vec![0; 16 * 16 * 4].into())
            }
        )
        .unwrap());
        assert_eq!(canvas.editor.revision(), revision);
        assert_eq!(
            canvas.editor.elements()[0].blur.as_ref().unwrap().sigma(),
            8
        );
    }

    #[test]
    fn annotation_only_change_rejects_ready_input_and_completed_work() {
        let mut canvas = canvas();
        let ready = completed(&canvas);
        accept(&mut canvas, ready).unwrap();
        let mut stale = completed(&canvas);
        let original = canvas.editor.layers().unwrap().clone();
        let mut changed = original.clone();
        changed.annotations = Some([255, 0, 0, 255].repeat(16 * 16).into());
        canvas
            .editor
            .replace_layers(canvas.editor.revision(), &original, changed)
            .unwrap();
        // 排除版本差异，仅靠完整层身份也必须拒绝旧结果和旧输入快照。
        stale.request.revision = canvas.editor.revision();
        assert!(!accept(&mut canvas, stale).unwrap());
        assert!(!canvas.pointer_down(crate::viewport::Point { x: 25.0, y: 25.0 }));
        canvas.sync_raster_source();
        assert!(canvas.blur_ready.is_none());
        assert!(canvas
            .image
            .as_ref()
            .unwrap()
            .pixels
            .same_storage(&original.base));
        let ready = completed(&canvas);
        assert!(accept(&mut canvas, ready).unwrap());
        assert!(canvas.pointer_down(crate::viewport::Point { x: 25.0, y: 25.0 }));
        canvas.cancel_gesture();
    }

    #[test]
    fn stale_source_version_tool_and_dimensions_never_install_result() {
        for case in 0..4 {
            let mut canvas = canvas();
            let mut result = completed(&canvas);
            match case {
                0 => result.request.source.base = vec![255; 16 * 16 * 4].into(),
                1 => result.request.revision = 1,
                2 => {
                    canvas.select_tool(Tool::Brush);
                }
                _ => result.request.area.width = 15,
            }
            assert!(!accept(&mut canvas, result).unwrap());
            assert!(canvas.blur_ready.is_none());
            assert!(canvas.editor.elements().is_empty());
        }
    }
}
