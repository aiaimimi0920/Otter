//! 图库的独立删除命中与单个悬浮阅读窗；预览不成为命令或键盘焦点页。
use super::*;
pub(super) struct PreviewWindow {
    pub level: usize,
    pub index: usize,
    pub bounds: RECT,
    pub panel: Panel,
}
impl Cascade {
    pub(super) fn retire_clicks(&mut self) {
        for page in &mut self.pages {
            page.pressed = None;
            page.delete_pressed = None;
        }
    }
    pub(crate) fn finish_command(&mut self, command: usize) {
        self.command = Some(command);
        self.cancel();
    }
    pub fn take_delete(&mut self) -> Option<usize> {
        self.pending_delete.take()
    }
    pub(super) fn delete_at(&self, level: usize, x: i32, y: i32) -> Option<usize> {
        let page = &self.pages[level];
        if page.layout.columns != 3 {
            return None;
        }
        let index = self.row_at(level, x, y)?;
        self.items(&page.path)[index].delete?;
        let rect = page
            .layout
            .delete_rect(index - page.first, self.content_width(level));
        (x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom).then_some(index)
    }
    pub(super) fn show_preview(&mut self, level: usize, index: Option<usize>) -> Result<()> {
        let Some(index) = index else {
            // 卡片间隙不撤去阅读面；离开图库/阅读面由屏幕坐标路由负责关闭。
            return Ok(());
        };
        if self
            .preview
            .as_ref()
            .is_some_and(|view| view.level == level && view.index == index)
        {
            return Ok(());
        }
        let page = &self.pages[level];
        let Some(content) = self.items(&page.path)[index].preview.clone() else {
            self.preview = None;
            return Ok(());
        };
        if let Some(view) = self
            .preview
            .as_mut()
            .filter(|view| view.level == level && view.panel.alive())
        {
            view.panel.set_preview(content);
            view.index = index;
            return Ok(());
        }
        let mut monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        unsafe {
            if !GetMonitorInfoW(
                MonitorFromRect(&page.bounds, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            )
            .as_bool()
            {
                return Err(Error::from_win32());
            }
        }
        let work = monitor.rcWork;
        let width = (self.row * 360 / 28).min(work.right - work.left);
        let height = (self.row * 320 / 28).min(work.bottom - work.top);
        let left = if page.bounds.right + width <= work.right {
            page.bounds.right
        } else {
            (page.bounds.left - width).max(work.left)
        };
        let top = page.bounds.top.clamp(work.top, work.bottom - height);
        let bounds = RECT {
            left,
            top,
            right: left + width,
            bottom: top + height,
        };
        let panel = Panel::create(self.owner, bounds, self.row, Vec::new())?;
        panel.set_preview(content);
        panel.show()?;
        self.preview = Some(PreviewWindow {
            level,
            index,
            bounds,
            panel,
        });
        Ok(())
    }
}
