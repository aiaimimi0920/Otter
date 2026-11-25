//! 同步实际 HWND 几何；外部移动不能继续使用创建时的屏幕命中矩形。
use super::*;

impl Cascade {
    pub(super) fn sync_geometry(&mut self) -> Result<()> {
        let mut moved = false;
        let mut preview_anchor_moved = false;
        let anchor = self.preview.as_ref().map(|view| view.level);
        for (level, page) in self.pages.iter_mut().enumerate() {
            if !page.panel.alive() {
                self.cancel();
                return Ok(());
            }
            let mut bounds = RECT::default();
            if let Err(error) = unsafe { GetWindowRect(page.panel.handle()?, &mut bounds) } {
                self.cancel();
                return Err(error);
            }
            if bounds == page.bounds {
                continue;
            }
            // 当前布局拥有固定字体/行高；外部改尺寸不能冒充已完成 DPI 重排。
            if bounds.right - bounds.left != page.bounds.right - page.bounds.left
                || bounds.bottom - bounds.top != page.bounds.bottom - page.bounds.top
            {
                self.cancel();
                return Ok(());
            }
            page.bounds = bounds;
            moved = true;
            preview_anchor_moved |= anchor == Some(level);
        }
        if self.preview.as_ref().is_some_and(|view| {
            let mut bounds = RECT::default();
            preview_anchor_moved
                || view.level >= self.pages.len()
                || view.panel.handle().map_or(true, |hwnd| unsafe {
                    GetWindowRect(hwnd, &mut bounds).is_err() || bounds != view.bounds
                })
        }) {
            // 外部移动、销毁或锚点换位后，旧预览矩形不能继续吞点击和滚轮。
            self.preview = None;
            moved = true;
        }
        if moved {
            self.pending_surrogate = None;
            for page in &mut self.pages {
                // 在旧坐标系开始的点击或滑块拖动不可跨越重定位执行。
                page.pressed = None;
                page.delete_pressed = None;
                page.drag = None;
                page.panel.discard_pointer_events();
            }
        }
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "native_menu_cascade_geometry_tests.rs"]
mod tests;
