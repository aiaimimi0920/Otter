//! 目标选择器的窄窗口：标准多选列表负责鼠标、键盘、滚动和可访问性，业务只消费稳定 ID。
use super::*;
#[path = "pin_targets_template.rs"]
mod layout;
#[path = "pin_targets_scroll.rs"]
mod scroll;
#[cfg(test)]
#[path = "pin_targets_window_tests.rs"]
mod tests;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows::{
    core::HSTRING,
    Win32::{
        Graphics::Gdi::*,
        UI::{
            Controls::*, HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::EnableWindow, Shell::*,
        },
    },
};
#[derive(Clone)]
pub(super) enum Event {
    All,
    Refresh,
    Confirm,
    Toggle(String, bool),
}
pub(super) struct Window(Rc<Model>);
struct Model {
    hwnd: Cell<Option<HWND>>,
    rows: RefCell<Vec<Entry>>,
    events: RefCell<VecDeque<Event>>,
    hover: Cell<Option<usize>>,
    status: RefCell<String>,
    brush: HBRUSH,
}
const LIST: i32 = 10;
const USER: i32 = (2 * std::mem::size_of::<isize>()) as i32;
impl Window {
    #[cfg(test)]
    pub(super) fn hwnd(&self) -> HWND {
        self.0.hwnd.get().unwrap()
    }
    pub(super) unsafe fn open(owner: HWND, rows: &[Entry], anchor: RECT) -> Result<Self> {
        let brush = CreateSolidBrush(color(neuro_mini_ui::Palette::default().surface));
        if brush.is_invalid() {
            return Err(windows::core::Error::from_win32());
        }
        let model = Rc::new(Model {
            hwnd: Cell::new(None),
            rows: RefCell::new(Vec::new()),
            events: RefCell::new(VecDeque::new()),
            hover: Cell::new(None),
            status: RefCell::new(String::new()),
            brush,
        });
        let template = layout::template();
        let hwnd = CreateDialogIndirectParamW(
            None,
            template.as_ptr().cast(),
            Some(owner),
            Some(procedure),
            LPARAM(Rc::as_ptr(&model) as isize),
        )?;
        let result = Self(model);
        let dpi = GetDpiForWindow(owner).max(96) as i32;
        let width = 176 * dpi / 96;
        let row = 28 * dpi / 96;
        let mut monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(
            MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST),
            &mut monitor,
        )
        .as_bool()
        {
            let _ = DestroyWindow(hwnd);
            return Err(windows::core::Error::from_win32());
        }
        let work = monitor.rcWork;
        let height =
            ((rows.len().clamp(1, 12) as i32 + 2) * row + 8 * dpi / 96).min(work.bottom - work.top);
        SetWindowPos(
            hwnd,
            None,
            anchor
                .left
                .clamp(work.left, (work.right - width).max(work.left)),
            anchor
                .bottom
                .clamp(work.top, (work.bottom - height).max(work.top)),
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        )?;
        let inset = 4 * dpi / 96;
        for (i, id) in [4, 3, 1].iter().enumerate() {
            let left = inset + (width - inset * 2) * i as i32 / 3;
            let right = inset + (width - inset * 2) * (i + 1) as i32 / 3;
            MoveWindow(
                GetDlgItem(Some(hwnd), *id)?,
                left,
                inset,
                right - left,
                row,
                true,
            )?;
        }
        let list = GetDlgItem(Some(hwnd), LIST)?;
        MoveWindow(
            list,
            inset,
            row + inset,
            width - inset * 2 - 8 * dpi / 96,
            height - row * 2 - inset * 2,
            true,
        )?;
        MoveWindow(
            GetDlgItem(Some(hwnd), scroll::ID)?,
            width - inset - 8 * dpi / 96,
            row + inset,
            8 * dpi / 96,
            height - row * 2 - inset * 2,
            true,
        )?;
        MoveWindow(
            GetDlgItem(Some(hwnd), 11)?,
            inset,
            height - row - inset,
            width - inset * 2,
            row,
            true,
        )?;
        SendMessageW(
            list,
            LB_SETITEMHEIGHT,
            Some(WPARAM(0)),
            Some(LPARAM(row as isize)),
        );
        result.update(rows, true, true, false);
        let _ = ShowWindow(hwnd, SW_SHOW);
        Ok(result)
    }
    pub(super) unsafe fn update(&self, rows: &[Entry], ready: bool, refresh: bool, applying: bool) {
        let Some(hwnd) = self.0.hwnd.get() else {
            return;
        };
        let Ok(list) = GetDlgItem(Some(hwnd), LIST) else {
            return;
        };
        // LISTBOX 在按下时选中、松开时才发 SELCHANGE；中途刷新不能撤回这次手势。
        if windows::Win32::UI::Input::KeyboardAndMouse::GetCapture() == list {
            return;
        }
        let same = {
            let old = self.0.rows.borrow();
            old.len() == rows.len()
                && old
                    .iter()
                    .zip(rows)
                    .all(|(a, b)| a.id == b.id && a.name == b.name)
        };
        if !same {
            let top = SendMessageW(list, LB_GETTOPINDEX, None, None);
            SendMessageW(list, WM_SETREDRAW, Some(WPARAM(0)), None);
            SendMessageW(list, LB_RESETCONTENT, None, None);
            *self.0.rows.borrow_mut() = rows.to_vec();
            for row in rows {
                let text = HSTRING::from(&row.name);
                SendMessageW(
                    list,
                    LB_ADDSTRING,
                    None,
                    Some(LPARAM(text.as_ptr() as isize)),
                );
            }
            SendMessageW(
                list,
                LB_SETTOPINDEX,
                Some(WPARAM(top.0.max(0) as usize)),
                None,
            );
            SendMessageW(list, WM_SETREDRAW, Some(WPARAM(1)), None);
            let _ = InvalidateRect(Some(list), None, false);
        }
        let changed = *self.0.rows.borrow() != rows;
        *self.0.rows.borrow_mut() = rows.to_vec();
        for (i, row) in rows.iter().enumerate() {
            let selected = SendMessageW(list, LB_GETSEL, Some(WPARAM(i)), None).0 > 0;
            if selected != row.checked {
                SendMessageW(
                    list,
                    LB_SETSEL,
                    Some(WPARAM(usize::from(row.checked))),
                    Some(LPARAM(i as isize)),
                );
            }
        }
        if changed {
            let _ = InvalidateRect(Some(list), None, false);
        }
        if !same {
            scroll::changed(list);
        }
        for (id, enabled) in [
            (4, !applying && rows.iter().any(|r| r.enabled)),
            (3, !applying && refresh),
            (1, !applying && ready),
            (LIST, !applying),
        ] {
            if let Ok(control) = GetDlgItem(Some(hwnd), id) {
                if windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled(control).as_bool()
                    != enabled
                {
                    let _ = EnableWindow(control, enabled);
                }
            }
        }
    }
    pub(super) fn event(&self) -> Option<Event> {
        self.0.events.borrow_mut().pop_front()
    }
    pub(super) unsafe fn status(&self, value: &str) {
        if *self.0.status.borrow() != value {
            *self.0.status.borrow_mut() = value.to_owned();
            if let Some(hwnd) = self.0.hwnd.get() {
                let _ = SetDlgItemTextW(hwnd, 11, &HSTRING::from(value));
            }
        }
    }
    pub(super) fn closed(&self) -> bool {
        self.0.hwnd.get().is_none()
    }
    pub(super) unsafe fn contains(&self, point: POINT) -> bool {
        self.0.hwnd.get().is_some_and(|hwnd| {
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect).is_ok()
                && point.x >= rect.left
                && point.x < rect.right
                && point.y >= rect.top
                && point.y < rect.bottom
        })
    }
    pub(super) unsafe fn translate(&self, message: &MSG) -> bool {
        self.0
            .hwnd
            .get()
            .is_some_and(|hwnd| IsDialogMessageW(hwnd, message).as_bool())
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if let Some(hwnd) = self.0.hwnd.get() {
                let _ = DestroyWindow(hwnd);
            }
        }
    }
}
impl Drop for Model {
    fn drop(&mut self) {
        unsafe {
            let _ = DeleteObject(self.brush.into());
        }
    }
}
fn color(rgb: u32) -> COLORREF {
    COLORREF(((rgb & 255) << 16) | (rgb & 0xff00) | (rgb >> 16))
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> isize {
    if msg == WM_INITDIALOG {
        let ptr = lp.0 as *const Model;
        Rc::increment_strong_count(ptr);
        SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), lp.0);
        (*ptr).hwnd.set(Some(hwnd));
        for id in [1, 3, 4] {
            if let Ok(button) = GetDlgItem(Some(hwnd), id) {
                let _ = crate::desktop::dialog_controls::button(button);
            }
        }
        if let Ok(list) = GetDlgItem(Some(hwnd), LIST) {
            let _ = SetWindowSubclass(list, Some(list_proc), 1, ptr as usize);
        }
        if let Ok(bar) = GetDlgItem(Some(hwnd), scroll::ID) {
            scroll::install(bar);
        }
        return 1;
    }
    let ptr = GetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER)) as *const Model;
    if ptr.is_null() {
        return 0;
    }
    Rc::increment_strong_count(ptr);
    let model = Rc::from_raw(ptr);
    match msg {
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            1
        }
        WM_NCDESTROY => {
            model.hwnd.set(None);
            SetWindowLongPtrW(hwnd, WINDOW_LONG_PTR_INDEX(USER), 0);
            drop(Rc::from_raw(ptr));
            0
        }
        WM_COMMAND => {
            let id = (wp.0 & 0xffff) as i32;
            let event = match id {
                4 => Some(Event::All),
                3 => Some(Event::Refresh),
                1 => Some(Event::Confirm),
                _ => None,
            };
            if let Some(event) = event {
                model.events.borrow_mut().push_back(event);
            } else if id == 2 {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            } else if id == LIST && (wp.0 >> 16) as u32 == LBN_SELCHANGE {
                if let Ok(list) = GetDlgItem(Some(hwnd), LIST) {
                    let rows = model.rows.borrow().clone();
                    for (i, row) in rows.iter().enumerate() {
                        let checked = SendMessageW(list, LB_GETSEL, Some(WPARAM(i)), None).0 > 0;
                        if checked != row.checked {
                            if row.enabled {
                                model.rows.borrow_mut()[i].checked = checked;
                                model
                                    .events
                                    .borrow_mut()
                                    .push_back(Event::Toggle(row.id.clone(), checked));
                            } else {
                                SendMessageW(
                                    list,
                                    LB_SETSEL,
                                    Some(WPARAM(usize::from(row.checked))),
                                    Some(LPARAM(i as isize)),
                                );
                            }
                        }
                    }
                    let _ = InvalidateRect(Some(list), None, false);
                }
            }
            1
        }
        WM_DRAWITEM => {
            draw(&model, &*(lp.0 as *const DRAWITEMSTRUCT));
            1
        }
        WM_CTLCOLORDLG | WM_CTLCOLORLISTBOX | WM_CTLCOLORSTATIC => {
            let dc = HDC(wp.0 as *mut _);
            let palette = neuro_mini_ui::Palette::default();
            let _ = SetTextColor(dc, color(palette.text));
            let _ = SetBkColor(dc, color(palette.surface));
            model.brush.0 as isize
        }
        _ => 0,
    }
}
unsafe extern "system" fn list_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    data: usize,
) -> LRESULT {
    let ptr = data as *const Model;
    Rc::increment_strong_count(ptr);
    let m = Rc::from_raw(ptr);
    if msg == WM_MOUSEMOVE {
        let hit = SendMessageW(hwnd, LB_ITEMFROMPOINT, None, Some(lp));
        let hover = ((hit.0 >> 16) == 0).then_some((hit.0 & 0xffff) as usize);
        if m.hover.replace(hover) != hover {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    } else if msg == WM_NCDESTROY {
        let _ = RemoveWindowSubclass(hwnd, Some(list_proc), id);
    }
    let scroll_input = matches!(
        msg,
        WM_MOUSEWHEEL | WM_KEYDOWN | WM_LBUTTONDOWN | WM_LBUTTONUP | WM_MOUSEMOVE | WM_TIMER
    );
    let before = scroll_input.then(|| SendMessageW(hwnd, LB_GETTOPINDEX, None, None));
    let result = DefSubclassProc(hwnd, msg, wp, lp);
    if before.is_some_and(|top| top != SendMessageW(hwnd, LB_GETTOPINDEX, None, None)) {
        scroll::changed(hwnd);
    }
    result
}
unsafe fn draw(m: &Model, item: &DRAWITEMSTRUCT) {
    if item.CtlID == scroll::ID as u32 {
        scroll::paint(item);
        return;
    }
    if item.CtlID != LIST as u32 {
        crate::desktop::dialog_controls::draw_action_button(item);
        return;
    }
    let palette = neuro_mini_ui::Palette::default();
    let row = m.rows.borrow().get(item.itemID as usize).cloned();
    let hovered = m.hover.get() == Some(item.itemID as usize);
    let active = item.itemState.0 & (ODS_SELECTED.0 | ODS_FOCUS.0) != 0 || hovered;
    let bg = if active {
        palette.control
    } else {
        palette.surface
    };
    SetDCBrushColor(item.hDC, color(bg));
    FillRect(item.hDC, &item.rcItem, HBRUSH(GetStockObject(DC_BRUSH).0));
    // 空列表的焦点通知使用 itemID == -1；没有行就只擦背景，不能传空 Vec 给 DrawTextW。
    let Some(row) = row else { return };
    let _ = SetBkMode(item.hDC, TRANSPARENT);
    let enabled = row.enabled;
    let _ = SetTextColor(
        item.hDC,
        color(if enabled { palette.text } else { palette.muted }),
    );
    let mut rect = item.rcItem;
    let mut text = {
        // 固定勾选框和文本起点，不能依赖不同 Unicode 字形的字宽。
        let dpi = GetDpiForWindow(item.hwndItem).max(96) as i32;
        let size = 14 * dpi / 96;
        let left = rect.left + 6 * dpi / 96;
        let top = (rect.top + rect.bottom - size) / 2;
        let mark = RECT {
            left,
            top,
            right: left + size,
            bottom: top + size,
        };
        SetDCBrushColor(
            item.hDC,
            color(if enabled { palette.text } else { palette.muted }),
        );
        FrameRect(item.hDC, &mark, HBRUSH(GetStockObject(DC_BRUSH).0));
        if row.checked {
            let mut check = "✓".encode_utf16().collect::<Vec<_>>();
            let mut bounds = mark;
            DrawTextW(
                item.hDC,
                &mut check,
                &mut bounds,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );
        }
        rect.left = mark.right + 6 * dpi / 96;
        row.name.encode_utf16().collect::<Vec<_>>()
    };
    if text.is_empty() {
        return;
    }
    DrawTextW(
        item.hDC,
        &mut text,
        &mut rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
    );
}
