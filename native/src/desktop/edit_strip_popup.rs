//! Preserve actual toolbar appearance while popups are open; original modal gates still own business exclusion, and open menus do not disable the entire bar.
use super::State;
use windows::{
    core::{w, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};

const ACTIVE: windows::core::PCWSTR = w!("Otter.EditStrip.PopupProjection");
pub(in super::super) unsafe fn active(state: &State) -> bool {
    GetPropW(state.tool_panel.get(), ACTIVE).0 == state as *const State as *mut _
}
struct Projection<'a> {
    state: &'a State,
    panel: HWND,
}
impl Drop for Projection<'_> {
    fn drop(&mut self) {
        unsafe {
            if self.state.tool_panel.get() == self.panel && active(self.state) {
                let _ = RemovePropW(self.panel, ACTIVE);
                super::strip::refresh(self.state);
            }
        }
    }
}
pub(in super::super) unsafe fn preserve<T>(
    state: &State,
    show: impl FnOnce() -> Result<T>,
) -> Result<T> {
    let panel = state.tool_panel.get();
    SetPropW(panel, ACTIVE, Some(HANDLE(state as *const State as *mut _)))?;
    let _projection = Projection { state, panel };
    show()
}
