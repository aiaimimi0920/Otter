//! 恢复替换只在无待交付工作时准备；不隐式取消执行或丢弃用户尚未提交的手势。
use super::Application;
pub(super) fn application(app: &Application) -> bool {
    !app.capture_active.get()
        && app.chrome.idle()
        && !app.projection_receive.preparing()
        && !app.projection_cleanup.pending_local()
        && !app.projection_sender.pending_local()
        && app.executions.is_idle()
        && app.parameter_file_job.is_none()
        && app.input_jobs.is_empty()
        && app.execution_jobs.is_empty()
        && !app.display_composition.borrow().is_busy()
        && !app.catalog.borrow().requested
        && !app.catalog.borrow().job.is_busy()
        && app.documents.iter().all(|doc| state(&doc.state))
        && !app
            .live_sessions
            .iter()
            .any(|session| session.art_pending())
}
pub(super) fn state(state: &crate::desktop::State) -> bool {
    if state.new_window.get()
        || state.art_request.borrow().is_some()
        || state.hotkey_toggle.get()
        || state.file_dialog.get()
        || state.new_pin.borrow().is_some()
        || state.group_request.borrow().is_some()
        || state.link_request.borrow().is_some()
        || state.link_creation.borrow().is_some()
        || state.parameter_file_request.borrow().is_some()
        || state.pending_adoption.borrow().is_some()
        || state.pending_execution.borrow().is_some()
        || state.pending_preparation.borrow().is_some()
        || state.pending_start.borrow().is_some()
        || state.execution_cancel.borrow().is_some()
        || state.execution_results.borrow().active().is_some()
        || state.copy_job.borrow().is_some()
        || state.export.borrow().is_some()
        || state.open_job.borrow().is_some()
        || state.crop_job.borrow().is_some()
        || state.rasterize_job.borrow().is_some()
        || state.blur_job.borrow().is_busy()
        || state.erase_job.borrow().is_busy()
        || state.resize_job.borrow().is_busy()
        || state.flip_job.borrow().is_busy()
    {
        return false;
    }
    let canvas = state.canvas.borrow();
    canvas.editor.draft().is_none()
        && canvas.drag.is_none()
        && canvas.draft_pointer.is_none()
        && canvas.crop_drag.is_none()
        && canvas.crop_pending.is_none()
        && canvas.path_drag.is_none()
        && canvas.rotation_drag.is_none()
        && canvas.scale_drag.is_none()
        && canvas.scale_pending.is_none()
        && canvas.scale_display_revision.is_none()
        && canvas.selection_drag.is_none()
        && canvas.marquee.is_none()
        && canvas.selection_effect_preview.is_none()
        && canvas.group_preview.is_empty()
        && canvas.erase_gesture.is_none()
        && canvas.erase_pending.is_none()
        && canvas.erase_display.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_requests_and_preview_gestures_block_without_being_consumed() {
        let source = crate::desktop::State::new(None, false).unwrap();
        assert!(state(&source));
        for flag in [
            &source.new_window,
            &source.hotkey_toggle,
            &source.file_dialog,
        ] {
            flag.set(true);
            assert!(!state(&source));
            assert!(flag.get());
            flag.set(false);
        }
        source.canvas.borrow_mut().selection_drag = Some((Default::default(), Default::default()));
        assert!(!state(&source));
        assert!(source.canvas.borrow().selection_drag.is_some());
        source.canvas.borrow_mut().selection_drag = None;
        source.canvas.borrow_mut().scale_display_revision = Some(0);
        assert!(!state(&source));
        source.canvas.borrow_mut().scale_display_revision = None;
        let request = crate::graph_requests::Request {
            id: crate::workspace_id::UnitId::parse("synthetic-request").unwrap(),
            generation: 1,
        };
        source.execution_results.borrow_mut().begin(request.clone());
        assert!(!state(&source));
        assert_eq!(source.execution_results.borrow().active(), Some(&request));
        assert!(source.execution_results.borrow_mut().cancel(&request));
        assert!(state(&source));
    }
}
