//! 自有 Win32 菜单面板；只绘制和收集有界输入，不执行业务命令。
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    marker::PhantomData,
    rc::Rc,
};
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::Threading::GetCurrentThreadId,
        UI::{Shell::*, WindowsAndMessaging::*},
    },
};
#[path = "native_menu_panel_uia.rs"]
mod accessibility;
#[path = "native_menu_panel_draw.rs"]
mod draw;
#[path = "native_menu_panel_font.rs"]
mod font;
#[path = "native_menu_panel_input.rs"]
mod input;
const ID: usize = 0x4f4d50;

#[derive(Clone)]
pub struct Row {
    /// 当前面板模型内的稳定条目索引；滚动后不能用可见槽位替代。
    pub key: usize,
    pub thumbnail: Option<crate::image_thumbnail::Thumbnail>,
    pub label: String,
    pub enabled: bool,
    pub checked: bool,
    pub submenu: bool,
    pub separator: bool,
    pub focused: bool,
    pub radio: bool,
    pub default: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Automation(usize, super::accessibility::Action),
    Move(i32, i32),
    Down(i32, i32),
    Up(i32, i32),
    Wheel(i16),
    Key(u32),
    Cancel,
    Dpi,
    ScrollTo(usize),
}
struct Data {
    notifications: RefCell<Option<super::uia::Snapshot>>,
    provider: RefCell<Option<windows::Win32::UI::Accessibility::IRawElementProviderSimple>>,
    accessibility: super::accessibility::Owner,
    bar: Cell<Option<super::scrollbar::Bar>>,
    rows: RefCell<Vec<Row>>,
    events: RefCell<VecDeque<Event>>,
    failed: Cell<bool>,
    destroyed: Cell<bool>,
    paints: Cell<u32>,
    row_height: i32,
    dpi: u32,
    fonts: font::Fonts,
}

/// 非激活 owned popup；级联控制器负责焦点路由、捕获、滚动及命令授权。
pub struct Panel {
    hwnd: HWND,
    data: Box<Data>,
    _thread: PhantomData<Rc<()>>,
}
impl Panel {
    pub(crate) fn preferred_width(owner: HWND, rows: &[Row]) -> Result<i32> {
        font::width(owner, rows)
    }
    /// 只供同线程控制器路由焦点/捕获；每次使用前重新验证实例身份。
    pub fn handle(&self) -> Result<HWND> {
        if self.alive() {
            Ok(self.hwnd)
        } else {
            Err(invalid())
        }
    }
    pub fn create(owner: HWND, bounds: RECT, row_height: i32, rows: Vec<Row>) -> Result<Self> {
        let width = bounds.right.checked_sub(bounds.left).ok_or_else(invalid)?;
        let height = bounds.bottom.checked_sub(bounds.top).ok_or_else(invalid)?;
        if width <= 0 || height <= 0 || !(1..=1024).contains(&row_height) || !valid(&rows) {
            return Err(invalid());
        }
        unsafe {
            if GetWindowThreadProcessId(owner, None) != GetCurrentThreadId() {
                return Err(invalid());
            }
            let data = Box::new(Data {
                notifications: RefCell::new(None),
                provider: RefCell::new(None),
                accessibility: super::accessibility::Owner::new(&rows),
                bar: Cell::new(None),
                rows: RefCell::new(rows),
                events: RefCell::new(VecDeque::new()),
                failed: Cell::new(false),
                destroyed: Cell::new(false),
                paints: Cell::new(0),
                row_height,
                dpi: windows::Win32::UI::HiDpi::GetDpiForWindow(owner).clamp(48, 768),
                fonts: font::Fonts::new(windows::Win32::UI::HiDpi::GetDpiForWindow(owner))?,
            });
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                w!("Otter 菜单"),
                WS_POPUP,
                bounds.left,
                bounds.top,
                width,
                height,
                Some(owner),
                None,
                None,
                None,
            )?;
            if !SetWindowSubclass(
                hwnd,
                Some(input::procedure),
                ID,
                (&*data as *const Data) as usize,
            )
            .as_bool()
            {
                let error = Error::from_win32();
                let _ = DestroyWindow(hwnd);
                return Err(error);
            }
            data.accessibility.set_location(hwnd.0 as usize, row_height);
            Ok(Self {
                hwnd,
                data,
                _thread: PhantomData,
            })
        }
    }
    pub fn show(&self) -> Result<()> {
        if !self.alive() {
            return Err(invalid());
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
    pub fn update(&self, rows: Vec<Row>) -> Result<()> {
        self.update_frame(rows, self.data.bar.get())
    }
    pub(crate) fn update_frame(
        &self,
        rows: Vec<Row>,
        bar: Option<super::scrollbar::Bar>,
    ) -> Result<()> {
        if !valid(&rows) || !self.alive() {
            return Err(invalid());
        }
        self.data.accessibility.replace(&rows);
        *self.data.rows.borrow_mut() = rows;
        self.data.bar.set(bar);
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
        Ok(())
    }
    pub fn next_event(&self) -> Option<Event> {
        if self.data.failed.get() || self.data.destroyed.get() {
            return Some(Event::Cancel);
        }
        self.data.events.borrow_mut().pop_front().or_else(|| {
            self.data
                .accessibility
                .take()
                .map(|(index, action)| match action {
                    super::accessibility::Action::ScrollTo(first) => Event::ScrollTo(first),
                    _ => Event::Automation(index, action),
                })
        })
    }
    /// 可撤销快照句柄；不暴露命令 ID，也不允许直接调用业务处理器。
    pub fn accessibility_nodes(&self) -> Vec<super::accessibility::Node> {
        if self.alive() {
            self.data.accessibility.nodes()
        } else {
            Vec::new()
        }
    }
    pub(crate) fn set_expanded(&self, key: Option<usize>) {
        self.data.accessibility.set_expanded(key);
    }
    pub(crate) fn set_scroll(&self, first: usize, visible: usize, total: usize) {
        self.data
            .accessibility
            .set_scroll(super::accessibility::Scroll {
                first,
                visible,
                total,
            });
    }
    pub(crate) fn clear_accessibility_links(&self) {
        self.data.accessibility.clear_links();
    }
    pub(crate) fn set_accessibility_active(&self, active: bool) {
        self.data.accessibility.set_active(active);
    }
    pub(crate) fn link_accessibility_child(&self, child: &Panel, key: usize) {
        self.data
            .accessibility
            .link_child(&child.data.accessibility, key);
    }
    pub(crate) fn discard_pointer_events(&self) {
        self.data.events.borrow_mut().retain(|event| {
            !matches!(
                event,
                Event::Move(..) | Event::Down(..) | Event::Up(..) | Event::Wheel(_)
            )
        });
    }
    pub fn alive(&self) -> bool {
        if self.data.destroyed.get() {
            return false;
        }
        let mut pointer = 0;
        unsafe {
            GetWindowSubclass(self.hwnd, Some(input::procedure), ID, Some(&mut pointer)).as_bool()
                && pointer == (&*self.data as *const Data) as usize
        }
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        // 核验子类实例身份，不能销毁复用同一 HWND 数值的新窗口。
        if self.alive() {
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
fn valid(rows: &[Row]) -> bool {
    rows.len() <= 64
        && rows.iter().all(|row| {
            row.label.len() <= 16384
                && row.label.encode_utf16().count() <= 4096
                && !row.label.contains('\0')
        })
}
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid native menu panel")
}

#[cfg(test)]
#[path = "native_menu_panel_tests.rs"]
mod tests;
