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
#[path = "native_menu_gallery_interaction.rs"]
mod gallery;
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
pub use modal::{track, track_compact, track_live, track_tray, track_tray_live};

struct Page {
    delete_pressed: Option<usize>,
    layout: super::layout::Layout,
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
    preview: Option<gallery::PreviewWindow>,
    pending_delete: Option<usize>,
    galleries: std::collections::HashMap<usize, Vec<Item>>,
    tree: Vec<Item>,
    pages: Vec<Page>,
    owner: HWND,
    tray_mode: bool,
    row: i32,
    width: i32,
    closed: bool,
    command: Option<usize>,
    checklist: bool,
    pending_surrogate: Option<u16>,
    navigation: navigation::State,
}
impl Cascade {
    pub fn open(owner: HWND, tree: Vec<Item>, point: POINT) -> Result<Self> {
        Self::open_with_tray_mode(owner, tree, point, false)
    }
    fn open_with_tray_mode(
        owner: HWND,
        tree: Vec<Item>,
        point: POINT,
        tray_mode: bool,
    ) -> Result<Self> {
        Self::open_sized(owner, tree, point, tray_mode, 320)
    }
    fn open_sized(
        owner: HWND,
        tree: Vec<Item>,
        point: POINT,
        tray_mode: bool,
        width: u32,
    ) -> Result<Self> {
        super::model::validate(&tree, None)?;
        let dpi = unsafe { GetDpiForWindow(owner) }.clamp(48, 768);
        let mut this = Self {
            preview: None,
            pending_delete: None,
            galleries: Default::default(),
            tree,
            pages: Vec::new(),
            owner,
            tray_mode,
            row: (28 * dpi / 96) as i32,
            width: (width * dpi / 96) as i32,
            closed: false,
            command: None,
            checklist: false,
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
        for (depth, &index) in path.iter().enumerate() {
            items = if depth == 0 {
                self.galleries
                    .get(&index)
                    .map(Vec::as_slice)
                    .unwrap_or_else(|| items[index].children.as_deref().unwrap_or(&[]))
            } else {
                items[index].children.as_deref().unwrap_or(&[])
            };
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
                deletable: item.delete.is_some(),
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
        let gallery = path.len() == 1 && self.galleries.contains_key(&path[0]);
        let layout = super::layout::Layout {
            columns: if gallery { 3 } else { 1 },
            row: if gallery {
                self.row * 80 / 28
            } else {
                self.row
            }
            .min(work.bottom - work.top)
            .max(1),
        };
        let visible = self
            .items(&path)
            .len()
            .div_ceil(layout.columns)
            .clamp(1, if gallery { 5 } else { 12 })
            .min(((work.bottom - work.top) / layout.row).max(1) as usize)
            * layout.columns;
        let desired = if gallery {
            self.width * 248 / 320
        } else {
            Panel::preferred_width(
                self.owner,
                &self.rows(&path, 0, self.items(&path).len(), None),
            )?
            .max(self.width)
        };
        let size = SIZE {
            cx: desired.min(work.right - work.left),
            cy: (visible / layout.columns) as i32 * layout.row,
        };
        // 只有托盘根页以鼠标为下角；子页仍按父条目顶边侧向展开。
        let alignment = if self.tray_mode && path.is_empty() {
            let horizontal = if i64::from(anchor.right) + i64::from(size.cx) > i64::from(work.right)
            {
                TPM_RIGHTALIGN
            } else {
                TPM_LEFTALIGN
            };
            TPM_BOTTOMALIGN | horizontal
        } else {
            TPM_TOPALIGN
        };
        let mut bounds = RECT::default();
        unsafe {
            CalculatePopupWindowPosition(
                &POINT {
                    x: anchor.right,
                    y: anchor.top,
                },
                &size,
                (TPM_VERTICAL | TPM_WORKAREA | alignment).0,
                Some(&anchor),
                &mut bounds,
            )?;
        }
        let panel = Panel::create(
            self.owner,
            bounds,
            layout.row,
            self.rows(&path, 0, visible, None),
        )?;
        panel.set_columns(layout.columns);
        self.pages.push(Page {
            delete_pressed: None,
            layout,
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
        if self.tray_mode {
            // 托盘浮层可能置顶；只提升本次菜单页，不改变隐藏业务 owner 的层级。
            unsafe {
                SetWindowPos(
                    self.pages[level].panel.handle()?,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER,
                )?;
            }
        }
        self.pages[level].panel.show()?;
        self.sync_accessibility();
        if self.tray_mode && level == 0 {
            self.pages[level].panel.announce_tray_popup()?;
        }
        Ok(())
    }
    fn refresh(&self, level: usize) -> Result<()> {
        self.refresh_frame(level, false)
    }
    fn refresh_frame(&self, level: usize, fresh: bool) -> Result<()> {
        let page = &self.pages[level];
        page.panel.set_scroll(
            page.first / page.layout.columns,
            page.visible / page.layout.columns,
            self.items(&page.path).len().div_ceil(page.layout.columns),
        );
        page.panel.update_frame(
            self.rows(&page.path, page.first, page.visible, page.focus),
            self.bar(level),
            fresh,
        )
    }
    fn focus(&mut self, level: usize, index: Option<usize>) -> Result<()> {
        if self.pages[level].focus == index {
            return Ok(());
        }
        self.pages.truncate(level + 1);
        if self.pages[level].layout.columns != 3 {
            self.preview = None;
        }
        self.pages[level].focus = index;
        self.pages[level].pressed = None;
        self.pages[level].delete_pressed = None;
        if let Some(index) = index {
            let page = &mut self.pages[level];
            if index < page.first {
                page.first = page.layout.align(index);
            }
            if index >= page.first + page.visible {
                page.first = page.layout.align(index) + page.layout.columns - page.visible;
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
        let rect = page
            .layout
            .rect(index - page.first, self.content_width(level));
        let anchor = RECT {
            left: page.bounds.left + rect.left,
            right: if page.layout.columns == 3 {
                page.bounds.right
            } else {
                page.bounds.left + rect.right
            },
            top: page.bounds.top + rect.top,
            bottom: page.bounds.top + rect.bottom,
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
    #[cfg(test)]
    pub(crate) fn open_checklist(owner: HWND, tree: Vec<Item>, point: POINT) -> Result<Self> {
        let mut menu = Self::open(owner, tree, point)?;
        menu.checklist = true;
        Ok(menu)
    }
    #[cfg(test)]
    pub(crate) fn replace_checklist(&mut self, tree: Vec<Item>) -> Result<()> {
        super::model::validate(&tree, None)?;
        if !self.checklist || self.closed {
            return Err(Error::new(E_ABORT, "Checklist closed"));
        }
        let page = &mut self.pages[0];
        let same_rows = self.tree.len() == tree.len()
            && self
                .tree
                .iter()
                .zip(&tree)
                .all(|(old, new)| old.label == new.label);
        if !same_rows {
            page.focus = None;
            page.pressed = None;
            page.panel.discard_pointer_events();
        }
        self.tree = tree;
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
            let work = monitor.rcWork;
            page.visible = self
                .tree
                .len()
                .clamp(1, 12)
                .min(((work.bottom - work.top) / page.layout.row).max(1) as usize);
            let height = page.visible as i32 * page.layout.row;
            page.bounds.top = page.bounds.top.min(work.bottom - height).max(work.top);
            page.bounds.bottom = page.bounds.top + height;
            SetWindowPos(
                page.panel.handle()?,
                None,
                page.bounds.left,
                page.bounds.top,
                page.bounds.right - page.bounds.left,
                height,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
        }
        page.first = page.first.min(self.tree.len().saturating_sub(page.visible));
        self.refresh_frame(0, !same_rows)?;
        self.sync_accessibility();
        Ok(())
    }
    pub fn cancel(&mut self) {
        self.preview = None;
        self.pending_delete = None;
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
                .find_map(|(level, page)| page.panel.next_event().map(|event| (level, event)))
                .or_else(|| {
                    self.preview.as_ref().and_then(|view| {
                        view.panel.next_event().map(|event| {
                            (
                                view.level,
                                match event {
                                    Event::WheelAt(..) | Event::Cancel | Event::Dpi => event,
                                    _ => Event::PreviewPointer,
                                },
                            )
                        })
                    })
                });
            let Some((level, event)) = next else {
                return Ok(());
            };
            if let Err(error) = self.event(level, event) {
                self.cancel();
                return Err(error);
            }
            if self.closed
                || (self.checklist && self.command.is_some())
                || self.navigation.pending.is_some()
                || self.pending_delete.is_some()
            {
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
