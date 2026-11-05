//! 钉图首次编辑前接管当前显示；不编辑来源窗口，不替换整个 Editor 或重置视口。
use super::{image::Image, State};
use crate::{
    editor::{Editor, Tool},
    erase_layers::Layers,
};

#[cfg(test)]
#[path = "display_edit_tests.rs"]
mod tests;

pub(super) struct Pending {
    transaction: crate::editor::Adoption,
    frame: Option<super::display_frame::Frame>,
}

/// 只能在事件处理返回后结算；后台编辑持有临时文档身份期间必须继续保留 owner。
pub(super) fn settle(state: &State) -> bool {
    if state.pending_adoption.borrow().is_none()
        || state.file_dialog.get()
        || state.blur_job.borrow().is_busy()
        || state.erase_job.borrow().is_busy()
        || state.crop_job.borrow().is_some()
        || state.rasterize_job.borrow().is_some()
        || state.resize_job.borrow().is_busy()
        || state.flip_job.borrow().is_busy()
    {
        return false;
    }
    let mut canvas = state.canvas.borrow_mut();
    if canvas.drag.is_some()
        || canvas.path_drag.is_some()
        || canvas.rotation_drag.is_some()
        || canvas.scale_drag.is_some()
        || canvas.selection_drag.is_some()
        || canvas.marquee.is_some()
        || canvas.crop_drag.is_some()
        || canvas.erase_gesture.is_some()
        || canvas.erase_pending.is_some()
        || canvas.crop_pending.is_some()
        || canvas.scale_pending.is_some()
    {
        return false;
    }
    let mut owner = state.pending_adoption.borrow_mut();
    let pending = owner.as_mut().unwrap();
    if pending.transaction.unchanged(&canvas.editor)
        && canvas.tool == Tool::Select
        && (canvas.selected.is_some() || canvas.multi_selection.is_some())
    {
        return false;
    }
    let selected = canvas.selected_indices();
    let outcome = match pending.transaction.finish(&mut canvas.editor) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("native-display-settle: {error:?}");
            return false;
        }
    };
    use crate::editor::AdoptionOutcome;
    if outcome == AdoptionOutcome::Pending {
        return false;
    }
    if outcome == AdoptionOutcome::Cancelled {
        *state.resolved_display.borrow_mut() = pending.frame.take();
    }
    owner.take();
    if outcome != AdoptionOutcome::Superseded {
        if outcome == AdoptionOutcome::Committed {
            // 最终内容与临时内容顺序相同，只更新文档版本，不丢失用户选择。
            let revision = canvas.editor.revision();
            canvas.selected = selected.first().map(|index| (*index, revision));
            canvas.multi_selection = (selected.len() > 1).then_some((selected, revision));
        } else {
            canvas.selected = None;
            canvas.multi_selection = None;
        }
        canvas.sync_raster_source();
        *state.hit_region.borrow_mut() = Default::default();
    }
    true
}

pub(super) fn prepare_text_at(state: &State, screen: crate::viewport::Point) -> bool {
    {
        let display = state.resolved_display.borrow();
        if let Some(frame) = display.as_ref() {
            let point = state.canvas.borrow().viewport.image_point(screen);
            let rect = frame
                .editor
                .as_deref()
                .and_then(|editor| editor.annotation_frame());
            let (x, y, width, height) = rect.map_or(
                (
                    0.0,
                    0.0,
                    frame.image.width as f32,
                    frame.image.height as f32,
                ),
                |r| (r.x as f32, r.y as f32, r.width as f32, r.height as f32),
            );
            if !point.x.is_finite()
                || !point.y.is_finite()
                || point.x < x
                || point.y < y
                || point.x >= x + width
                || point.y >= y + height
            {
                return false;
            }
        }
    }
    prepare_for(state, Intent::Text)
}

pub(super) fn prepare(state: &State) -> bool {
    prepare_for(state, Intent::Pointer)
}

pub(super) fn prepare_content(state: &State) -> bool {
    prepare_for(state, Intent::Content)
}

pub(super) fn prepare_crop(state: &State) -> bool {
    prepare_for(state, Intent::Crop)
}

#[derive(PartialEq, Eq)]
enum Intent {
    Pointer,
    Text,
    Content,
    Crop,
}

fn prepare_for(state: &State, intent: Intent) -> bool {
    if state.canvas.borrow().minified.is_some() {
        return false;
    }
    let art = state.is_art();
    if art
        && (intent == Intent::Content
            || (intent == Intent::Pointer
                && matches!(
                    state.canvas.borrow().tool,
                    Tool::Eraser | Tool::Blur | Tool::Mosaic
                )))
    {
        return false;
    }
    if state.group_locked.get()
        || state.file_dialog.get()
        || state.erase_job.borrow().is_busy()
        || state.crop_job.borrow().is_some()
        || state.rasterize_job.borrow().is_some()
        || state.resize_job.borrow().is_busy()
        || state.flip_job.borrow().is_busy()
    {
        return false;
    }
    if intent == Intent::Pointer
        && state.canvas.borrow().tool == Tool::Pan
        && !state.canvas.borrow().crop_mode
    {
        return true;
    }
    if state.pending_adoption.borrow().is_some() {
        return true;
    }
    let mut display = state.resolved_display.borrow_mut();
    let Some(frame) = display.as_ref() else {
        return true;
    };
    let mut canvas = state.canvas.borrow_mut();
    if art && !frame.is_from(&state.unit_id) && !frame.has_overlay_owner(&state.unit_id) {
        return false;
    }
    let fallback;
    let source = if art {
        let Ok(editor) = canvas.editor.export_overlay_with_base(
            Layers {
                size: (frame.image.width, frame.image.height),
                base: frame.image.pixels.clone(),
                annotations: None,
                annotation_size: None,
            },
            |text| {
                unsafe { super::annotation_text::measure(text) }
                    .map_err(|_| crate::editor::EditError::InvalidPoint)
            },
        ) else {
            return false;
        };
        fallback = editor;
        &fallback
    } else if let Some(editor) = frame.editor.as_deref() {
        editor
    } else {
        let Image {
            width,
            height,
            pixels,
        } = &frame.image;
        let Ok(editor) = Editor::with_layers(Layers {
            size: (*width, *height),
            base: pixels.clone(),
            annotations: None,
            annotation_size: None,
        }) else {
            return false;
        };
        fallback = editor;
        &fallback
    };
    match canvas.editor.begin_adoption(source) {
        Ok(Some(transaction)) => {
            canvas.cancel_gesture();
            canvas.clear_scale_display();
            canvas.selected = None;
            canvas.multi_selection = None;
            canvas.blur_ready = None;
            canvas.sync_raster_source();
            *state.pending_adoption.borrow_mut() = Some(Pending {
                transaction,
                frame: display.take(),
            });
            *state.hit_region.borrow_mut() = Default::default();
            true
        }
        Ok(None) => false,
        Err(error) => {
            eprintln!("native-display-edit: {error:?}");
            false
        }
    }
}
