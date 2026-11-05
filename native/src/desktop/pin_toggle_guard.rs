//! 缩略与展开共用切换准入；拒绝时不取消任务、消费请求或丢弃用户草稿。
use super::State;

pub(super) fn allows(state: &State) -> bool {
    if state.group_locked.get()
        || state.group_hidden.get()
        || state.file_dialog.get()
        || state.capture_active.get()
        || state.exit_requested.get()
        || state.pending_adoption.borrow().is_some()
        || state.pending_execution.borrow().is_some()
        || state.pending_preparation.borrow().is_some()
        || state.pending_start.borrow().is_some()
        || state.execution_cancel.borrow().is_some()
        || state.execution_results.borrow().active().is_some()
        || super::super::group_lock::editing_busy(state)
    {
        return false;
    }
    let canvas = state.canvas.borrow();
    canvas.pinned
        && !canvas.blocks_wheel()
        && !canvas.crop_mode
        && canvas.drag.is_none()
        && canvas.draft_pointer.is_none()
        && canvas.crop_pending.is_none()
        && canvas.erase_pending.is_none()
        && canvas.erase_display.is_none()
        && canvas.scale_pending.is_none()
        && canvas.scale_display_revision.is_none()
        && canvas.selection_effect_preview.is_none()
        && canvas.group_preview.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editor::Tool, viewport::Point};
    #[test]
    fn toggle_guard_preserves_blocking_flags_and_uncommitted_input() {
        let state = State::new(None, true).unwrap();
        assert!(allows(&state));
        for flag in [
            &state.group_locked,
            &state.group_hidden,
            &state.file_dialog,
            state.capture_active.as_ref(),
            state.exit_requested.as_ref(),
        ] {
            flag.set(true);
            assert!(!allows(&state));
            assert!(flag.get());
            flag.set(false);
        }
        state
            .canvas
            .borrow_mut()
            .editor
            .begin(Tool::Brush, Point { x: 1.0, y: 1.0 })
            .unwrap();
        assert!(!allows(&state));
        assert!(state.canvas.borrow().editor.draft().is_some());
        state.canvas.borrow_mut().cancel_gesture();
        assert!(allows(&state));
        state.canvas.borrow_mut().scale_display_revision = Some(0);
        assert!(!allows(&state));
        assert_eq!(state.canvas.borrow().scale_display_revision, Some(0));
        state.canvas.borrow_mut().scale_display_revision = None;
        state.canvas.borrow_mut().tool = Tool::Text;
        assert!(allows(&state));
    }
    #[test]
    fn toggle_guard_does_not_cancel_running_execution() {
        let state = State::new(None, true).unwrap();
        let request = crate::graph_requests::Request {
            id: crate::workspace_id::UnitId::parse("toggle-test").unwrap(),
            generation: 1,
        };
        state.execution_results.borrow_mut().begin(request.clone());
        assert!(!allows(&state));
        assert_eq!(state.execution_results.borrow().active(), Some(&request));
        assert!(state.execution_results.borrow_mut().cancel(&request));
        assert!(allows(&state));
    }
}
