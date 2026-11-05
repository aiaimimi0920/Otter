//! 组变换中心模式只属于当前窗口；切换取消预览和异步候选，不修改文档历史。
use super::State;
use windows::{
    core::{w, Result},
    Win32::UI::WindowsAndMessaging::*,
};
pub(super) const COMMAND: usize = 0x1024;
pub(super) const AXIS_FREE: usize = 0x1025;
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
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let flags = MF_STRING
        | if state.canvas.borrow().transform_own {
            MF_CHECKED
        } else {
            MF_UNCHECKED
        };
    AppendMenuW(menu, flags, COMMAND, w!("分别围绕各标注自身中心变换"))?;
    for (id, axis, text) in [
        (AXIS_FREE, MoveAxis::Free, w!("移动：自由")),
        (AXIS_X, MoveAxis::X, w!("移动：仅水平")),
        (AXIS_Y, MoveAxis::Y, w!("移动：仅垂直")),
    ] {
        let flags = MF_STRING
            | if state.canvas.borrow().move_axis == axis {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            };
        AppendMenuW(menu, flags, id, text)?;
    }
    for (id, axis, text) in [
        (ROTATION_FREE, MoveAxis::Free, w!("旋转：自由角度")),
        (ROTATION_X, MoveAxis::X, w!("旋转：水平拖动")),
        (ROTATION_Y, MoveAxis::Y, w!("旋转：垂直拖动")),
    ] {
        let flags = MF_STRING
            | if state.canvas.borrow().rotation_axis == axis {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            };
        AppendMenuW(menu, flags, id, text)?;
    }
    for (id, axis, text) in [
        (SCALE_FREE, MoveAxis::Free, w!("缩放：自由")),
        (SCALE_X, MoveAxis::X, w!("缩放：仅宽度")),
        (SCALE_Y, MoveAxis::Y, w!("缩放：仅高度")),
    ] {
        let flags = MF_STRING
            | if state.canvas.borrow().scale_axis == axis {
                MF_CHECKED
            } else {
                MF_UNCHECKED
            };
        AppendMenuW(menu, flags, id, text)?;
    }
    Ok(())
}
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
