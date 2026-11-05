//! UIA 实时屏幕几何；先同步视口，沿用绘制/输入的滚动投影与裁剪，不创建 D2D target。
use super::*;
use windows::Win32::{
    Graphics::Gdi::ClientToScreen,
    UI::{Accessibility::*, WindowsAndMessaging::*},
};

fn unavailable() -> Error {
    Error::from_hresult(windows::core::HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32))
}
impl Host {
    pub(in crate::desktop) fn slider_readonly(&self, id: &str) -> Option<bool> {
        self.edits.slider_readonly(id)
    }
    pub(in crate::desktop) fn take_slider_readonly_changes(&mut self) -> Vec<(String, bool, bool)> {
        self.edits.take_slider_readonly_changes()
    }
    pub(in crate::desktop) fn slider_value(&self, id: &str) -> Option<f64> {
        self.edits.slider_value(id)
    }
    pub(in crate::desktop) fn take_slider_changes(&mut self) -> Vec<(String, f64, f64)> {
        self.edits.take_slider_changes()
    }
    pub(in crate::desktop) fn accessibility_slider_range(
        &mut self,
        id: &str,
    ) -> Result<(crate::surface_slider::Range, f64, bool)> {
        if !self.sync_viewport()? {
            return Err(unavailable());
        }
        self.edits.slider_range(id)
    }
    pub(in crate::desktop) fn accessibility_set_slider_range(
        &mut self,
        id: &str,
        value: f64,
    ) -> Result<()> {
        self.edits.commit_slider_range(id, value)
    }
    pub(in crate::desktop) fn take_edit_structure_change(&mut self) -> bool {
        self.edits.take_structure_change()
    }
    pub(in crate::desktop) fn accessibility_edit_hwnd(&self, id: &str) -> Option<HWND> {
        self.edits.hwnd(id)
    }
    pub(in crate::desktop) fn accessibility_edit_id(&self, hwnd: HWND) -> Option<String> {
        self.edits.id(hwnd).map(str::to_owned)
    }
    pub(in crate::desktop) fn take_switch_changes(&mut self) -> Vec<(String, bool, bool)> {
        self.edits.take_switch_changes()
    }
    pub(in crate::desktop) fn accessibility_switch_value(&mut self, id: &str) -> Result<bool> {
        if !self.sync_viewport()? {
            return Err(unavailable());
        }
        self.switch_value(id).ok_or_else(unavailable)
    }
    /// 调用方已同步/显露并复查权限；提交点不再进入 Win32，避免检查后重入。
    pub(in crate::desktop) fn accessibility_commit_toggle(&mut self, id: &str) -> Result<bool> {
        self.edits.toggle_switch(id)
    }
    pub(in crate::desktop) fn accessibility_can_focus(&mut self, id: &str) -> Result<bool> {
        Ok(self.sync_viewport()? && self.presentation.can_focus(id))
    }
    pub(in crate::desktop) fn accessibility_reveal_focus(&mut self, id: &str) -> Result<bool> {
        if !self.sync_viewport()? {
            return Ok(false);
        }
        let ready = self.presentation.reveal_focus(id)?;
        if ready {
            self.sync_edits()?;
        }
        Ok(ready)
    }
    pub(in crate::desktop) fn accessibility_bounds(&mut self, id: &str) -> Result<(UiaRect, bool)> {
        if !self.sync_viewport()? {
            return Err(unavailable());
        }
        let node = self
            .presentation
            .geometry()
            .nodes()
            .iter()
            .find(|node| node.id() == id)
            .ok_or_else(unavailable)?;
        let mut origin = POINT::default();
        if !unsafe { ClientToScreen(self.hwnd, &mut origin) }.as_bool() {
            return Err(Error::from_win32());
        }
        let scale = self.viewport.1 as f64 / 96.;
        // BoundingRectangle 不截去部分不可见区域；可见性单独按祖先裁剪和 HWND 判断。
        let rect = UiaRect {
            left: origin.x as f64 + node.origin()[0] as f64 * scale,
            top: origin.y as f64 + node.origin()[1] as f64 * scale,
            width: node.size()[0] as f64 * scale,
            height: node.size()[1] as f64 * scale,
        };
        let offscreen = node.clip().empty()
            || unsafe { !IsWindowVisible(self.hwnd).as_bool() || IsIconic(self.hwnd).as_bool() };
        Ok((rect, offscreen))
    }
    pub(in crate::desktop) fn accessibility_hit(
        &mut self,
        screen: [f64; 2],
    ) -> Result<Option<String>> {
        if !self.sync_viewport()? {
            return Err(unavailable());
        }
        let mut origin = POINT::default();
        if !unsafe { ClientToScreen(self.hwnd, &mut origin) }.as_bool() {
            return Err(Error::from_win32());
        }
        let scale = 96. / self.viewport.1 as f64;
        let point = [
            (screen[0] - origin.x as f64) * scale,
            (screen[1] - origin.y as f64) * scale,
        ];
        Ok(self
            .presentation
            .hit_test(point.map(|value| value as f32))
            .map(str::to_owned))
    }
}
