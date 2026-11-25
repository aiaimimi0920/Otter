//! Owned Win32 menu panels only draw/collect bounded input, never execute business commands.
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
#[path = "native_menu_owner_wheel.rs"]
mod owner_wheel;
#[path = "native_menu_panel_preview.rs"]
mod preview;
#[path = "native_menu_panel_text.rs"]
mod text;
pub use preview::Preview;
const ID: usize = 0x4f4d50;

#[derive(Clone)]
pub struct Row {
    pub deletable: bool,
    /// Stable item index within the current panel model; visible slots cannot replace it after scrolling.
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
    PreviewPointer,
    WheelAt(i16, i32, i32),
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
    _paint_session: crate::desktop::buffered_paint::Session,
    delete_hover: Cell<Option<usize>>,
    preview: RefCell<Option<Preview>>,
    columns: Cell<usize>,
    notifications: RefCell<Option<super::uia::Snapshot>>,
    provider: RefCell<Option<windows::Win32::UI::Accessibility::IRawElementProviderSimple>>,
    accessibility: super::accessibility::Owner,
    bar: Cell<Option<super::scrollbar::Bar>>,
    rows: RefCell<text::PaintRows>,
    events: RefCell<VecDeque<Event>>,
    failed: Cell<bool>,
    destroyed: Cell<bool>,
    tray_popup: Cell<bool>,
    paints: Cell<u32>,
    row_height: i32,
    dpi: u32,
    fonts: font::Fonts,
}

/// Nonactivating owned popup; cascade controllers own focus routing, capture, scrolling, and command authorization.
pub struct Panel {
    hwnd: HWND,
    data: Box<Data>,
    _thread: PhantomData<Rc<()>>,
}
impl Panel {
    pub(crate) fn preferred_width(owner: HWND, rows: &[Row]) -> Result<i32> {
        font::width(owner, rows)
    }
    /// Only same-thread controllers may route focus/capture; revalidate instance identity before every use.
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
            let dpi = windows::Win32::UI::HiDpi::GetDpiForWindow(owner).clamp(48, 768);
            let data = Box::new(Data {
                _paint_session: crate::desktop::buffered_paint::Session::new()?,
                delete_hover: Cell::new(None),
                preview: RefCell::new(None),
                columns: Cell::new(1),
                notifications: RefCell::new(None),
                provider: RefCell::new(None),
                accessibility: super::accessibility::Owner::new(&rows),
                bar: Cell::new(None),
                rows: RefCell::new(text::PaintRows::new(rows)),
                events: RefCell::new(VecDeque::new()),
                failed: Cell::new(false),
                destroyed: Cell::new(false),
                tray_popup: Cell::new(false),
                paints: Cell::new(0),
                row_height,
                dpi,
                fonts: font::Fonts::new(dpi)?,
            });
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("STATIC"),
                &windows::core::HSTRING::from(crate::ui_text::text("native-menu-title")),
                // STATIC defaults to hit-test transparency, routing system wheels behind it; menus must be hittable.
                WS_POPUP | WINDOW_STYLE(windows::Win32::System::SystemServices::SS_NOTIFY.0),
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
    pub(crate) fn announce_tray_popup(&self) -> Result<()> {
        // Custom-drawn tray roots must also notify Shell of menu lifecycles; notifications can synchronously reenter window destruction.
        if self.notification_guard(|| {
            if !self.data.tray_popup.replace(true) {
                unsafe {
                    windows::Win32::UI::Accessibility::NotifyWinEvent(
                        EVENT_SYSTEM_MENUPOPUPSTART,
                        self.hwnd,
                        OBJID_CLIENT.0,
                        0,
                    );
                }
            }
        }) {
            Ok(())
        } else {
            Err(Error::new(E_ABORT, "Tray menu changed during notification"))
        }
    }
    pub fn update(&self, rows: Vec<Row>) -> Result<()> {
        self.update_frame(rows, self.data.bar.get(), false)
    }
    pub(crate) fn set_columns(&self, columns: usize) {
        assert!(matches!(columns, 1 | 3));
        self.data.columns.set(columns);
        self.data.accessibility.set_columns(columns);
    }
    pub(crate) fn set_delete_hover(&self, key: Option<usize>) {
        if self.data.delete_hover.replace(key) != key {
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn delete_hover(&self) -> Option<usize> {
        self.data.delete_hover.get()
    }
    pub(crate) fn update_frame(
        &self,
        rows: Vec<Row>,
        bar: Option<super::scrollbar::Bar>,
        fresh: bool,
    ) -> Result<()> {
        if !valid(&rows) || !self.alive() {
            return Err(invalid());
        }
        // Data replacement atomically retires old UIA nodes/queued actions; ordinary focus redraw still permits Focus -> Invoke.
        if fresh {
            self.data.accessibility.replace_fresh(&rows);
        } else {
            self.data.accessibility.replace(&rows);
        }
        let mut bounds = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut bounds)?;
        }
        self.data
            .accessibility
            .set_content_width(bar.map_or(bounds.right, |bar| bar.track.left));
        self.data.rows.borrow_mut().replace(rows);
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
    pub(super) fn update_thumbnails(
        &self,
        image: impl Fn(usize) -> Option<crate::image_thumbnail::Thumbnail>,
    ) -> Result<()> {
        if !self.alive() {
            return Err(invalid());
        }
        for row in &mut self.data.rows.borrow_mut().rows {
            row.thumbnail = image(row.key);
        }
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
        Ok(())
    }
    /// Revocable snapshot handle; exposes neither command IDs nor direct business-handler invocation.
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
                Event::Move(..)
                    | Event::Down(..)
                    | Event::Up(..)
                    | Event::Wheel(_)
                    | Event::WheelAt(..)
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
impl Data {
    fn end_tray_popup(&self, hwnd: HWND) {
        // Retire the flag first; hiding/destruction, including notification reentry, sends only one end notice.
        if self.tray_popup.replace(false) {
            unsafe {
                windows::Win32::UI::Accessibility::NotifyWinEvent(
                    EVENT_SYSTEM_MENUPOPUPEND,
                    hwnd,
                    OBJID_CLIENT.0,
                    0,
                );
            }
        }
    }
}
impl Drop for Panel {
    fn drop(&mut self) {
        // Check subclass-instance identity; never destroy a new window reusing the same HWND value.
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
