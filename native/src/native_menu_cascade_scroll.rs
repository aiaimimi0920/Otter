//! 根菜单捕获保持不变；滑块拖动只拥有页面偏移，不可产生行命令。
use super::*;
use crate::native_menu::scrollbar::Bar;
impl Cascade {
    pub(super) fn bar(&self, level: usize) -> Option<Bar> {
        let page = &self.pages[level];
        Bar::new(
            page.bounds.right - page.bounds.left,
            page.bounds.bottom - page.bounds.top,
            self.row,
            self.items(&page.path).len(),
            page.visible,
            page.first,
        )
    }
    pub(super) fn dragging(&self) -> Option<usize> {
        self.pages.iter().position(|page| page.drag.is_some())
    }
    pub(super) fn set_first(&mut self, level: usize, first: usize) -> Result<()> {
        self.pages.truncate(level + 1);
        let max = self
            .items(&self.pages[level].path)
            .len()
            .saturating_sub(self.pages[level].visible);
        let page = &mut self.pages[level];
        page.first = first.min(max);
        page.focus = None;
        page.pressed = None;
        self.refresh(level)
    }
    pub(super) fn scroll_event(&mut self, level: usize, event: Event) -> Result<bool> {
        let Some(bar) = self.bar(level) else {
            return Ok(false);
        };
        if let Some((start, first)) = self.pages[level].drag {
            match event {
                Event::Move(_, y) => {
                    self.set_first(level, bar.drag(start, first, y))?;
                    return Ok(true);
                }
                Event::Up(..) => {
                    self.pages[level].drag = None;
                    return Ok(true);
                }
                Event::Down(..) | Event::Wheel(_) => return Ok(true),
                _ => self.pages[level].drag = None,
            }
        }
        match event {
            Event::Down(x, y) if bar.hit(x, y) => {
                self.pages.truncate(level + 1);
                self.pages[level].pressed = None;
                if bar.in_thumb(y) {
                    self.pages[level].drag = Some((y, self.pages[level].first));
                    self.focus(level, None)?;
                } else {
                    let page = &self.pages[level];
                    let next = if y < bar.thumb.top {
                        page.first.saturating_sub(page.visible)
                    } else {
                        page.first.saturating_add(page.visible)
                    };
                    self.set_first(level, next)?;
                }
                Ok(true)
            }
            Event::Move(x, y) if bar.hit(x, y) => {
                self.focus(level, None)?;
                Ok(true)
            }
            Event::Up(x, y) if bar.hit(x, y) => {
                self.pages[level].pressed = None;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
#[path = "native_menu_cascade_scroll_tests.rs"]
mod tests;
