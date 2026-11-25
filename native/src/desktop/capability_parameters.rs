//! Native advanced-parameter editing; do not borrow state across modal message loops, and bind commits to selection identity and local revision.
use super::State;
use crate::loom_parameter_state::Parameters;
use serde_json::{Map, Value};
#[cfg(test)]
use windows::core::w;
use windows::{
    core::Result,
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x1082;
pub(super) const MAX_TEXT: usize = 1024 * 1024;
struct Edit {
    stamp: super::configuration_stamp::Stamp,
    candidate: Parameters,
}
pub(super) fn parse(text: &str) -> Option<Map<String, Value>> {
    if text.len() > MAX_TEXT {
        return None;
    }
    serde_json::from_str(text).ok()
}
impl Edit {
    fn capture(state: &State) -> Option<Self> {
        let selected = state.selected_art.borrow();
        let selected = selected.as_ref()?;
        Some(Self {
            stamp: super::configuration_stamp::Stamp::capture(state),
            candidate: selected.parameters.clone(),
        })
    }
    fn text(&self) -> Result<String> {
        let value: Map<String, Value> = self
            .candidate
            .entries()
            .filter_map(|(id, value)| value.map(|v| (id.into(), v.clone())))
            .collect();
        // Use compact JSON so formatting whitespace cannot inflate accepted state beyond the input limit.
        let text = serde_json::to_string(&value).map_err(|_| invalid())?;
        if text.len() > MAX_TEXT {
            return Err(invalid());
        }
        Ok(text)
    }
    unsafe fn commit(mut self, hwnd: HWND, state: &State, text: &str) -> Result<bool> {
        if !IsWindow(Some(hwnd)).as_bool() || state.group_locked.get() {
            return Ok(false);
        }
        let changed = self
            .candidate
            .replace(parse(text).ok_or_else(invalid)?)
            .map_err(|_| invalid())?;
        if !changed {
            return Ok(false);
        }
        if !self.stamp.matches(state) {
            return Ok(false);
        }
        let mut selection = state.selected_art.borrow_mut();
        let Some(selected) = selection.as_mut() else {
            return Ok(false);
        };
        // PostMessage runs no nested message loop; retain old parameters if cancellation cannot be enqueued.
        super::execution_cancel::request(hwnd, state)?;
        selected.parameters = self.candidate;
        drop(selection);
        super::execution_start::parameter_changed(state);
        Ok(true)
    }
}
fn invalid() -> windows::core::Error {
    windows::core::Error::new(
        E_INVALIDARG,
        crate::ui_text::text("integration-invalid-parameters"),
    )
}
pub(super) unsafe fn show(hwnd: HWND, state: &State) -> Result<()> {
    let Some(_dialog) = super::file_dialog::Guard::begin(&state.file_dialog) else {
        return Ok(());
    };
    let Some(edit) = Edit::capture(state) else {
        return Ok(());
    };
    if let Some(text) = super::shape_dialog::show_parameters(hwnd, edit.text()?) {
        edit.commit(hwnd, state, &text)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "capability_parameters_tests.rs"]
mod tests;

#[cfg(test)]
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    AppendMenuW(
        menu,
        MF_STRING
            | if state.selected_art.borrow().is_some() {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        COMMAND,
        &windows::core::HSTRING::from(crate::ui_text::text("integration-advanced-parameters")),
    )
}
