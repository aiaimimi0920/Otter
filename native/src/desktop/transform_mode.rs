//! 组变换中心模式只属于当前窗口；切换取消预览和异步候选，不修改文档历史。
use super::State;
pub(super) const COMMAND: usize = 0x1024;
pub(super) const AXIS_X: usize = 0x1026;
pub(super) const AXIS_Y: usize = 0x1027;
pub(super) const ROTATION_FREE: usize = 0x1028;
pub(super) const ROTATION_X: usize = 0x1029;
pub(super) const ROTATION_Y: usize = 0x102A;
pub(super) const SCALE_FREE: usize = 0x102B;
pub(super) const SCALE_X: usize = 0x102C;
pub(super) const SCALE_Y: usize = 0x102D;

pub(super) fn dispatch(state: &State, command: usize) {
    if command == COMMAND {
        toggle(state);
    } else if (ROTATION_FREE..=ROTATION_Y).contains(&command) {
        let axis = match command {
            ROTATION_X => MoveAxis::X,
            ROTATION_Y => MoveAxis::Y,
            _ => MoveAxis::Free,
        };
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        canvas.rotation_axis = axis;
    } else if (SCALE_FREE..=SCALE_Y).contains(&command) {
        let axis = match command {
            SCALE_X => MoveAxis::X,
            SCALE_Y => MoveAxis::Y,
            _ => MoveAxis::Free,
        };
        state.resize_job.borrow().cancel();
        let mut canvas = state.canvas.borrow_mut();
        canvas.cancel_gesture();
        canvas.scale_axis = axis;
        canvas.clear_scale_display();
    } else {
        set_axis(state, command);
    }
}

pub(super) use crate::transform_gesture::Axis as MoveAxis;
pub(super) fn set_axis(state: &State, command: usize) {
    let axis = match command {
        AXIS_X => MoveAxis::X,
        AXIS_Y => MoveAxis::Y,
        _ => MoveAxis::Free,
    };
    let mut canvas = state.canvas.borrow_mut();
    canvas.cancel_gesture();
    canvas.move_axis = axis;
}
pub(super) fn toggle(state: &State) {
    state.resize_job.borrow().cancel();
    let mut canvas = state.canvas.borrow_mut();
    canvas.cancel_gesture();
    canvas.transform_own = !canvas.transform_own;
    canvas.clear_scale_display();
}
