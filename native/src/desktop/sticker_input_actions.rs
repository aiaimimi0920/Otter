//! 参数窗和内嵌image控件共用事务；模态返回复核owner，不授予路径/网络读取的隐式权限。
use super::super::parameter_file_input::Outcome;
use super::super::{configuration_stamp::Stamp, State};
use windows::{core::Result, Win32::Foundation::HWND};
#[derive(Clone, Copy)]
pub(in crate::desktop) enum Action {
    Upload,
    Clear,
    ToggleDisabled,
}
pub(in crate::desktop) unsafe fn apply(
    hwnd: HWND,
    state: &State,
    stamp: &Stamp,
    action: Action,
    owner_current: &dyn Fn() -> bool,
) -> Result<Outcome> {
    let current = || {
        owner_current()
            && stamp.matches(state)
            && !state.is_art()
            && state.lifetime.owns(hwnd)
            && super::super::document_window::parameter_panel::available(state)
    };
    if !current() || state.file_dialog.get() {
        return Ok(Outcome::Rejected);
    }
    match action {
        Action::Upload => {
            return super::super::parameter_file_input::choose_target(
                hwnd,
                state,
                super::super::parameter_file_input::Target::Sticker,
                stamp.clone(),
                None,
                &current,
            )
        }
        Action::Clear => state.sticker_input.set_asset(None)?,
        Action::ToggleDisabled => state.sticker_input.toggle()?,
    }
    Ok(Outcome::Accepted)
}
