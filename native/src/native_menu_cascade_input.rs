//! 级联导航与可见行映射；按下/抬起必须命中同一当前条目才返回命令。
use super::*;
impl Cascade {
    fn row_at(&self, level: usize, x: i32, y: i32) -> Option<usize> {
        let page = &self.pages[level];
        if x < 0
            || y < 0
            || x >= page.bounds.right - page.bounds.left
            || y >= page.bounds.bottom - page.bounds.top
        {
            return None;
        }
        let index = page.first + (y / self.row) as usize;
        self.items(&page.path)
            .get(index)
            .filter(|item| item.enabled && !item.separator)
            .map(|_| index)
    }
    pub(super) fn activate(&mut self, level: usize, index: usize) -> Result<()> {
        if self.navigation.pending.is_some() {
            return Ok(());
        }
        let item = &self.items(&self.pages[level].path)[index];
        if !item.enabled || item.separator {
            return Ok(());
        }
        if item.children.is_some() {
            self.expand(level, index)
        } else {
            let command = item.command;
            let page = &self.pages[level];
            let Some(slot) = index.checked_sub(page.first) else {
                self.cancel();
                return Ok(());
            };
            if !page.panel.notify_invoked(slot) || self.pages.iter().any(|page| !page.panel.alive())
            {
                self.cancel();
                return Ok(());
            }
            if self.navigation.commands.contains(&command) {
                self.navigation.pending = Some(command);
                return Ok(());
            }
            self.command = Some(command);
            self.cancel();
            Ok(())
        }
    }
    pub(super) fn navigate(&mut self, level: usize, direction: isize) -> Result<()> {
        let page = &self.pages[level];
        let items = self.items(&page.path);
        let count = items.len();
        if count == 0 {
            return Ok(());
        }
        let start = page
            .focus
            .map(|index| index as isize)
            .unwrap_or(if direction > 0 { -1 } else { 0 });
        let found = (1..=count)
            .map(|step| (start + direction * step as isize).rem_euclid(count as isize) as usize)
            .find(|&index| items[index].enabled && !items[index].separator);
        if let Some(index) = found {
            self.focus(level, Some(index))?;
        }
        Ok(())
    }
    pub(super) fn event(&mut self, level: usize, event: Event) -> Result<()> {
        self.pending_surrogate = None; // 导航/指针事件不能把前一菜单的半个字符带到新目标。
        if self.scroll_event(level, event)? {
            self.sync_accessibility();
            return Ok(());
        }
        match event {
            Event::ScrollTo(first) => self.set_first(level, first)?,
            Event::Automation(visible, action) => {
                use crate::native_menu::accessibility::Action;
                let page = &self.pages[level];
                if visible >= page.visible {
                    return Ok(());
                }
                let index = page.first + visible;
                let Some(item) = self.items(&page.path).get(index) else {
                    return Ok(());
                };
                if !item.enabled || item.separator {
                    return Ok(());
                }
                match action {
                    Action::Focus => {
                        // 同一父行再次获取焦点也须退出子页，不能被 focus 的同值快路吞掉。
                        self.pages.truncate(level + 1);
                        self.pages[level].pressed = None;
                        self.pages[level].drag = None;
                        self.focus(level, Some(index))?;
                    }
                    Action::Invoke if item.children.is_none() => self.activate(level, index)?,
                    Action::Expand if item.children.is_some() => {
                        self.focus(level, Some(index))?;
                        self.expand(level, index)?;
                    }
                    Action::Collapse
                        if item.children.is_some()
                            && self
                                .pages
                                .get(level + 1)
                                .is_some_and(|child| child.path.last() == Some(&index)) =>
                    {
                        self.pages.truncate(level + 1);
                    }
                    _ => {}
                }
            }
            Event::Move(x, y) => {
                let index = self.row_at(level, x, y);
                self.focus(level, index)?;
                if let Some(index) = index {
                    self.expand(level, index)?;
                }
            }
            Event::Down(x, y) => self.pages[level].pressed = self.row_at(level, x, y),
            Event::Up(x, y) => {
                let pressed = self.pages[level].pressed.take();
                if let Some(index) =
                    pressed.filter(|index| Some(*index) == self.row_at(level, x, y))
                {
                    self.activate(level, index)?;
                }
            }
            Event::Key(0x26) => self.navigate(level, -1)?,
            Event::Key(0x28) => self.navigate(level, 1)?,
            Event::Key(0x0d | 0x20 | 0x27) => {
                if let Some(index) = self.pages[level].focus {
                    if matches!(event, Event::Key(0x27)) {
                        self.expand(level, index)?;
                    } else {
                        self.activate(level, index)?;
                    }
                    if !self.closed && self.pages.len() > level + 1 {
                        self.navigate(level + 1, 1)?;
                    }
                }
            }
            Event::Key(0x25 | 0x1b) => {
                if level > 0 {
                    self.pages.truncate(level);
                } else if matches!(event, Event::Key(0x1b)) {
                    self.cancel();
                }
            }
            Event::Wheel(delta) => {
                let mut lines = 3u32;
                unsafe {
                    let _ = SystemParametersInfoW(
                        SPI_GETWHEELSCROLLLINES,
                        0,
                        Some((&mut lines as *mut u32).cast()),
                        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS::default(),
                    );
                }
                let count = self.items(&self.pages[level].path).len();
                self.pages.truncate(level + 1);
                let page = &mut self.pages[level];
                page.wheel += i32::from(delta);
                let notches = page.wheel / 120;
                page.wheel %= 120;
                let step = if lines == u32::MAX {
                    page.visible as i64
                } else {
                    i64::from(lines)
                };
                let first = (page.first as i64 - i64::from(notches) * step)
                    .clamp(0, count.saturating_sub(page.visible) as i64)
                    as usize;
                if first != page.first {
                    page.first = first;
                    page.focus = None;
                    page.pressed = None;
                    self.refresh(level)?;
                }
            }
            Event::Cancel | Event::Dpi => self.cancel(), // DPI 切换先安全关闭，后续重建字体/布局。
            _ => {}
        }
        self.sync_accessibility();
        Ok(())
    }
}
