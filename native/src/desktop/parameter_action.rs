//! 参数菜单动作的局部候选；模态返回不直接提交，调用方重新核对配置凭证。
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
    /// 外层 None 是取消；Some(None) 是恢复未设置，不得转换成显式 null。
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
