//! 选中模糊强度的精确输入；模态返回后核验原文档、选择和强度，再提交既有后台任务。
use super::{canvas::Canvas, numeric_style::Kind, State};
use crate::{blur_job::Request, editor::DocumentStamp};
use windows::{
    core::w,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x1049;
#[cfg(test)]
#[path = "blur_numeric_tests.rs"]
mod tests;
struct Pending {
    stamp: DocumentStamp,
    index: usize,
    strength: f32,
}
impl Pending {
    fn capture(canvas: &Canvas, previous: Option<&Request>) -> Option<Self> {
        Some(Self {
            stamp: canvas.editor.document_stamp(),
            index: canvas.selected_index()?,
            strength: super::blur_selection::current(canvas, previous)?,
        })
    }
    fn request(self, canvas: &Canvas, previous: Option<&Request>, text: &str) -> Option<Request> {
        if !self.stamp.matches(&canvas.editor)
            || canvas.selected_index() != Some(self.index)
            || super::blur_selection::current(canvas, previous)? != self.strength
        {
            return None;
        }
        super::blur_selection::absolute(canvas, previous, Kind::BlurStrength.parse(text)?)
    }
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> windows::core::Result<()> {
    let enabled = super::blur_input::selected_strength(&state.canvas.borrow()).is_some();
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        COMMAND,
        w!("选中模糊强度…"),
    )
}
pub(super) unsafe fn show(hwnd: HWND, state: &State) {
    if !super::tool_panel_availability::Availability::capture(state)
        .is_some_and(|value| value.allows(COMMAND))
    {
        return;
    }
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return;
    };
    let previous = state.blur_job.borrow().current_request().cloned();
    let Some(pending) = Pending::capture(&state.canvas.borrow(), previous.as_ref()) else {
        return;
    };
    let value =
        super::shape_dialog::show_numeric(hwnd, pending.strength.to_string(), Kind::BlurStrength);
    if !IsWindow(Some(hwnd)).as_bool() {
        return;
    }
    if let Some(value) = value {
        let previous = state.blur_job.borrow().current_request().cloned();
        let request = pending.request(&state.canvas.borrow(), previous.as_ref(), &value);
        if let Some(request) = request {
            super::blur_input::submit(hwnd, state, request);
        }
    }
}
