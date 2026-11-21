//! 序号后续创建半径的原生入口；仅变更会话工具配置，不创建编辑历史。
use super::State;
use crate::editor::Tool;
use windows::{
    core::w,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x1020;
pub(super) unsafe fn show(hwnd: HWND, state: &State) {
    if state.canvas.borrow().minified.is_some() {
        return;
    }
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let radius = {
        let canvas = state.canvas.borrow();
        if canvas.tool != Tool::Serial {
            return;
        }
        canvas.tool_profiles.serial_radius()
    };
    let Some(value) = super::shape_dialog::show_serial_radius(hwnd, radius.to_string()) else {
        return;
    };
    if !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    let Some(radius) = crate::shape_value::step(&value).filter(|value| (8..=999).contains(value))
    else {
        MessageBoxW(
            Some(hwnd),
            w!("序号半径必须为 8–999 图像像素，原设置未修改。"),
            w!("Otter"),
            MB_OK | MB_ICONERROR,
        );
        return;
    };
    let mut canvas = state.canvas.borrow_mut();
    if canvas.tool == Tool::Serial {
        canvas.tool_profiles.set_serial_radius(radius);
    }
}
