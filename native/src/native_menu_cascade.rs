//! 自有菜单面板的级联控制器；只返回命令 ID，业务授权仍由调用者完成。
use super::{
    panel::{Event, Panel, Row},
    Item,
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};
#[path = "native_menu_cascade_geometry.rs"]
mod geometry;
#[path = "native_menu_cascade_input.rs"]
mod input;
#[path = "native_menu_mnemonic.rs"]
mod mnemonic;
#[path = "native_menu_modal.rs"]
mod modal;
#[path = "native_menu_navigation.rs"]
mod navigation;
#[path = "native_menu_cascade_scroll.rs"]
mod scroll;
#[path = "native_menu_cascade_update.rs"]
mod update;
pub use modal::{track, track_live, track_tray, track_tray_live};

struct Page {
    path: Vec<usize>,
    panel: Panel,
    bounds: RECT,
    first: usize,
    visible: usize,
    focus: Option<usize>,
    pressed: Option<usize>,
    wheel: i32,
    drag: Option<(i32, usize)>,
}
pub struct Cascade {
    tree: Vec<Item>,
    pages: Vec<Page>,
    owner: HWND,
    row: i32,
    width: i32,
    closed: bool,
    command: Option<usize>,
    pending_surrogate: Option<u16>,
    navigation: navigation::State,
}
impl Cascade {
    pub fn open(owner: HWND, tree: Vec<Item>, point: POINT) -> Result<Self> {
        super::model::validate(&tree, None)?;
        let dpi = unsafe { GetDpiForWindow(owner) }.clamp(48, 768);
        let mut this = Self {
            tree,
            pages: Vec::new(),
            owner,
            row: (28 * dpi / 96) as i32,
            width: (320 * dpi / 96) as i32,
            closed: false,
            command: None,
            pending_surrogate: None,
            navigation: Default::default(),
        };
        this.add(
            Vec::new(),
            RECT {
                left: point.x,
                right: point.x,
                top: point.y,
                bottom: point.y,
            },
        )?;
        Ok(this)
    }
    fn items(&self, path: &[usize]) -> &[Item] {
        let mut items = self.tree.as_slice();
        for &index in path {
            items = items[index].children.as_deref().unwrap_or(&[]);
        }
        items
    }
    fn rows(&self, path: &[usize], first: usize, visible: usize, focus: Option<usize>) -> Vec<Row> {
        self.items(path)
            .iter()
            .enumerate()
            .skip(first)
            .take(visible)
            .map(|(index, item)| Row {
                key: index,
                thumbnail: item.thumbnail.clone(),
                label: item.label.clone(),
                enabled: item.enabled,
                checked: item.checked,
                submenu: item.children.is_some(),
                separator: item.separator,
                focused: focus == Some(index),
                radio: item.radio,
                default: item.default,
            })
            .collect()
    }
    fn add(&mut self, path: Vec<usize>, anchor: RECT) -> Result<()> {
        if self.pages.len() >= 16 {
            return Err(Error::new(E_INVALIDARG, "Menu nesting limit"));
        }
        let mut monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        unsafe {
            if !GetMonitorInfoW(
                MonitorFromRect(&anchor, MONITOR_DEFAULTTONEAREST),
                &mut monitor,
            )
            .as_bool()
            {
                return Err(Error::from_win32());
            }
        }
        let work = monitor.rcWork;
        let visible = self
            .items(&path)
            .len()
            .clamp(1, 12)
            .min(((work.bottom - work.top) / self.row).max(1) as usize);
        let desired = Panel::preferred_width(
            self.owner,
            &self.rows(&path, 0, self.items(&path).len(), None),
        )?;
        let size = SIZE {
            cx: desired.max(self.width).min(work.right - work.left),
            cy: (visible as i32 * self.row).min(work.bottom - work.top),
        };
        let mut bounds = RECT::default();
        unsafe {
            CalculatePopupWindowPosition(
                &POINT {
                    x: anchor.right,
                    y: anchor.top,
                },
                &size,
                (TPM_VERTICAL | TPM_WORKAREA).0,
                Some(&anchor),
                &mut bounds,
            )?;
        }
        let panel = Panel::create(
            self.owner,
            bounds,
            self.row,
            self.rows(&path, 0, visible, None),
        )?;
        self.pages.push(Page {
            path,
            panel,
            bounds,
            first: 0,
            visible,
            focus: None,
            pressed: None,
            wheel: 0,
            drag: None,
        });
        let level = self.pages.len() - 1;
        self.refresh(level)?;
        self.pages[level].panel.show()?;
        self.sync_accessibility();
        Ok(())
    }
    fn refresh(&self, level: usize) -> Result<()> {
        let page = &self.pages[level];
        page.panel
            .set_scroll(page.first, page.visible, self.items(&page.path).len());
        page.panel.update_frame(
            self.rows(&page.path, page.first, page.visible, page.focus),
            self.bar(level),
        )
    }
    fn focus(&mut self, level: usize, index: Option<usize>) -> Result<()> {
        if self.pages[level].focus == index {
            return Ok(());
        }
        self.pages.truncate(level + 1);
        self.pages[level].focus = index;
        self.pages[level].pressed = None;
        if let Some(index) = index {
            let page = &mut self.pages[level];
            if index < page.first {
                page.first = index;
            }
            if index >= page.first + page.visible {
                page.first = index + 1 - page.visible;
            }
        }
        self.refresh(level)
    }
    fn expand(&mut self, level: usize, index: usize) -> Result<()> {
        if self.pages.len() > level + 1 && self.pages[level + 1].path.last() == Some(&index) {
            return Ok(());
        }
        let page = &self.pages[level];
        let item = &self.items(&page.path)[index];
        if !item.enabled || item.separator || item.children.is_none() {
            return Ok(());
        }
        let mut path = page.path.clone();
        path.push(index);
        let anchor = RECT {
            top: page.bounds.top + (index - page.first) as i32 * self.row,
            bottom: page.bounds.top + (index - page.first + 1) as i32 * self.row,
            ..page.bounds
        };
        self.pages.truncate(level + 1);
        self.add(path, anchor)
    }
    pub fn closed(&self) -> bool {
        self.closed
    }
    fn sync_accessibility(&self) {
        let active = self.pages.iter().rposition(|page| page.focus.is_some());
        for page in &self.pages {
            page.panel.clear_accessibility_links();
        }
        for (level, page) in self.pages.iter().enumerate() {
            page.panel.set_accessibility_active(active == Some(level));
            if let Some(child) = self.pages.get(level + 1) {
                if let Some(&key) = child.path.last() {
                    page.panel.link_accessibility_child(&child.panel, key);
                }
            }
            page.panel.set_expanded(
                self.pages
                    .get(level + 1)
                    .and_then(|child| child.path.last())
                    .copied(),
            );
        }
        // 全部关系及焦点提交后才通知；UIA 回调不得观察一半旧、一半新的树。
        for page in &self.pages {
            page.panel.notify_accessibility();
        }
    }
    pub fn take_command(&mut self) -> Option<usize> {
        self.command.take()
    }
    pub fn cancel(&mut self) {
        self.navigation.pending = None;
        self.closed = true;
        self.pages.clear();
    }
    /// 外层消息循环每次 dispatch 后调用；没有私有业务消息或后台内容指针。
    pub fn poll(&mut self) -> Result<()> {
        self.sync_geometry()?;
        self.sync_accessibility();
        if self.closed {
            return Ok(());
        }
        for _ in 0..128 {
            let next = self
                .pages
                .iter()
                .enumerate()
                .find_map(|(level, page)| page.panel.next_event().map(|event| (level, event)));
            let Some((level, event)) = next else {
                return Ok(());
            };
            if let Err(error) = self.event(level, event) {
                self.cancel();
                return Err(error);
            }
            if self.closed || self.navigation.pending.is_some() {
                return Ok(());
            }
        }
        self.cancel(); // 有界处理；输入过载不能继续执行不完整手势。
        Ok(())
    }
    /// 非激活窗口的硬件键盘消息须由外层菜单消息循环路由到此处。
    pub fn key(&mut self, key: u32) -> Result<()> {
        self.sync_geometry()?;
        if self.closed || self.pages.is_empty() {
            return Ok(());
        }
        if self.pages.iter().any(|page| !page.panel.alive()) {
            self.cancel();
            return Ok(());
        }
        let result = self.event(self.pages.len() - 1, Event::Key(key));
        if result.is_err() {
            self.cancel();
        }
        result
    }
}

#[cfg(test)]
#[path = "native_menu_cascade_tests.rs"]
mod tests;
