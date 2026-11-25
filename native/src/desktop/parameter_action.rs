//! Local candidates for parameter-menu actions; modal return does not commit directly, and callers recheck configuration evidence.
use super::{parameter_value::Spec, State};
use serde_json::Value;
use windows::Win32::Foundation::HWND;
pub(super) enum Action {
    Set(Value),
    Reset(Option<Value>),
    ToggleDisabled(Value),
    Edit(Spec, String),
    Color(String),
    File,
}
impl Action {
    /// Outer None is cancellation; Some(None) restores unset state and must not become explicit null.
    pub unsafe fn choose(&self, hwnd: HWND, state: &State) -> Option<Option<Value>> {
        if let Self::Set(value) = self {
            return Some(Some(value.clone()));
        }
        if let Self::Reset(value) = self {
            return Some(value.clone());
        }
        if matches!(self, Self::ToggleDisabled(_) | Self::File) {
            return None;
        }
        let _guard = super::file_dialog::Guard::begin(&state.file_dialog)?;
        match self {
            Self::Edit(spec, text) => {
                super::parameter_dialog::show(hwnd, spec.clone(), text.clone()).map(Some)
            }
            Self::Color(text) => {
                let initial = super::color_dialog::parse_hex(text)
                    .unwrap_or(crate::stroke_style::Color(0, 0, 0));
                super::color_dialog::choose(hwnd, initial)
                    .map(|color| Some(Value::String(super::color_dialog::hex(color))))
            }
            Self::Set(_) | Self::Reset(_) | Self::ToggleDisabled(_) | Self::File => unreachable!(),
        }
    }
}
