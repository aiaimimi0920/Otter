//! JS 回复只在同一前台 Surface 内切焦；不激活窗口、不保存裸 HWND、不跨 SetFocus 借用 Host。
use super::*;
use crate::surface_javascript_wire::focus::Request;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, IsWindowEnabled, SetFocus};

pub(super) unsafe fn observe(hwnd: HWND, state: &State, message: u32, wp: WPARAM, lp: LPARAM) {
    let native_edge = matches!(message, WM_SETFOCUS | WM_KILLFOCUS)
        || (message == WM_COMMAND
            && matches!((wp.0 >> 16) as u32, EN_SETFOCUS | EN_KILLFOCUS)
            && GetParent(HWND(lp.0 as *mut _)).ok() == Some(hwnd));
    if native_edge {
        // 饱和后拒绝所有请求，不能回绕复活旧帧；Host 暂时取出时仍记录系统边沿。
        state
            .focus_epoch
            .set(state.focus_epoch.get().saturating_add(1));
    }
}
impl SurfaceWindow {
    pub(crate) fn javascript_focus_epoch(&self) -> u64 {
        self.state.focus_epoch.get()
    }
    pub(super) fn javascript_focus_allowed(&self) -> bool {
        if !self.local_edit_enabled() || self.state.host.borrow().is_none() {
            return false;
        }
        unsafe {
            if !IsWindowVisible(self.hwnd).as_bool()
                || !IsWindowEnabled(self.hwnd).as_bool()
                || IsIconic(self.hwnd).as_bool()
                || GetForegroundWindow() != self.hwnd
            {
                return false;
            }
            let focused = GetFocus();
            focused == self.hwnd
                || self
                    .state
                    .host
                    .borrow()
                    .as_ref()
                    .is_some_and(|host| host.accessibility_edit_id(focused).is_some())
        }
    }
    pub(crate) fn apply_javascript_focus(
        &self,
        requests: &[Request],
        mut epoch: u64,
    ) -> Result<()> {
        crate::surface_javascript_wire::focus::validate(requests)
            .map_err(|_| Error::new(E_INVALIDARG, "Invalid JavaScript focus request"))?;
        for request in requests {
            if epoch == u64::MAX
                || epoch != self.javascript_focus_epoch()
                || !self.javascript_focus_allowed()
            {
                break;
            }
            let Some(handle) = self.accessibility_node(&request.target) else {
                continue;
            };
            if !handle.read().is_some_and(|entry| {
                entry.kind == crate::surface_scene::Kind::Input && !entry.disabled
            }) {
                continue;
            }
            let mut target = None;
            self.state.run(|host| {
                if !host.accessibility_bounds(&request.target)?.1 {
                    target = host.accessibility_edit_hwnd(&request.target);
                }
                Ok(())
            })?;
            let Some(target) = target else {
                continue;
            };
            if epoch != self.javascript_focus_epoch()
                || !self.javascript_focus_allowed()
                || handle.read().is_none()
            {
                break;
            }
            unsafe {
                if GetParent(target).ok() != Some(self.hwnd)
                    || !IsWindowEnabled(target).as_bool()
                    || !IsWindowVisible(target).as_bool()
                {
                    continue;
                }
                if (!request.focus && GetFocus() != target)
                    || (request.focus && GetFocus() == target)
                {
                    continue;
                }
                let expected = if request.focus { target } else { self.hwnd };
                // Host 已归还：EN_KILLFOCUS 继续进入原草稿/change FIFO，不手造第二次提交。
                let _ = SetFocus(Some(expected));
                if !self.is_alive() || handle.read().is_none() || GetFocus() != expected {
                    break;
                }
                keyboard::set_focus(&self.state, request.focus.then(|| request.target.clone()));
                epoch = self.javascript_focus_epoch();
            }
        }
        Ok(())
    }
}
