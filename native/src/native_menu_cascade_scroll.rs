//! 根菜单捕获保持不变；滑块拖动只拥有页面偏移，不可产生行命令。
use super::*;
use crate::native_menu::scrollbar::Bar;
impl Cascade {
    pub(super) fn content_width(&self, level: usize) -> i32 {
        self.bar(level).map_or(
            self.pages[level].bounds.right - self.pages[level].bounds.left,
            |bar| bar.track.left,
        )
    }
    pub(super) fn gallery_key(&mut self, level: usize, direction: isize) -> Result<()> {
        let page = &self.pages[level];
        let count = self.items(&page.path).len();
        let Some(index) = page.focus else {
            return self.navigate(level, direction.signum());
        };
        let target =
            (index as isize + direction).clamp(0, count.saturating_sub(1) as isize) as usize;
        if self.items(&page.path)[target].enabled {
            self.focus(level, Some(target))?;
        }
        Ok(())
    }
    pub(super) fn bar(&self, level: usize) -> Option<Bar> {
        let page = &self.pages[level];
        Bar::new(
            page.bounds.right - page.bounds.left,
            page.bounds.bottom - page.bounds.top,
            page.layout.row.min(self.row),
            self.items(&page.path).len().div_ceil(page.layout.columns),
            page.visible / page.layout.columns,
            page.first / page.layout.columns,
        )
    }
    pub(super) fn dragging(&self) -> Option<usize> {
        self.pages.iter().position(|page| page.drag.is_some())
    }
    pub(super) fn set_first(&mut self, level: usize, first: usize) -> Result<()> {
        self.pages.truncate(level + 1);
        let count = self.items(&self.pages[level].path).len();
        let page = &mut self.pages[level];
        page.panel.set_delete_hover(None);
        page.first = page
            .layout
            .align(first)
            .min(page.layout.max_first(count, page.visible));
        page.focus = None;
        page.pressed = None;
        page.delete_pressed = None;
        self.refresh(level)
    }
    pub(super) fn scroll_event(&mut self, level: usize, event: Event) -> Result<bool> {
        let Some(bar) = self.bar(level) else {
            return Ok(false);
        };
        if let Some((start, first)) = self.pages[level].drag {
            match event {
                Event::Move(_, y) => {
                    self.set_first(
                        level,
                        bar.drag(start, first, y) * self.pages[level].layout.columns,
                    )?;
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
                self.pages[level].panel.set_delete_hover(None);
                self.pages.truncate(level + 1);
                self.pages[level].pressed = None;
                self.pages[level].delete_pressed = None;
                if bar.in_thumb(y) {
                    self.pages[level].drag = Some((
                        y,
                        self.pages[level].first / self.pages[level].layout.columns,
                    ));
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
                self.pages[level].delete_pressed = None;
                self.pages[level].panel.set_delete_hover(None);
                self.focus(level, None)?;
                Ok(true)
            }
            Event::Up(x, y) if bar.hit(x, y) => {
                self.pages[level].pressed = None;
                self.pages[level].delete_pressed = None;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "native_menu_cascade_scroll_tests.rs"]
mod tests;
