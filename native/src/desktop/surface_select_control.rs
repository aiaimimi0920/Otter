//! 单线程 COMBOBOX owner；仅拥有选项 HWND/索引，不授权、不提交、不维护表单 draft。
use crate::surface_select::Options;
use std::{marker::PhantomData, rc::Rc};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::Threading::GetCurrentThreadId,
        UI::{
            Input::KeyboardAndMouse::{EnableWindow, GetFocus, IsWindowEnabled, SetFocus},
            WindowsAndMessaging::*,
        },
    },
};
const OWNER: windows::core::PCWSTR = w!("Otter.Surface.Select.Owner");
#[path = "surface_select_color.rs"]
mod color;
use super::surface_control_font as font;
#[path = "surface_select_hover.rs"]
mod hover;
#[path = "surface_select_items.rs"]
mod items;
#[path = "surface_select_messages.rs"]
mod messages;
#[path = "surface_select_paint.rs"]
mod paint;
#[path = "surface_select_text.rs"]
mod text;
/// 参数面板复用同一标准COMBOBOX像素层，不接入Surface字段或动作状态。
pub(super) unsafe fn paint_standard(hwnd: HWND, message: u32, wp: WPARAM) -> Option<LRESULT> {
    paint::message(hwnd, message, wp)
}
pub struct Control {
    hwnd: HWND,
    marker: Box<u8>,
    options: Options,
    _thread: PhantomData<Rc<()>>,
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn size(rect: RECT) -> Result<(i32, i32)> {
    let width = rect
        .right
        .checked_sub(rect.left)
        .ok_or_else(|| invalid("Select bounds overflow"))?;
    let height = rect
        .bottom
        .checked_sub(rect.top)
        .ok_or_else(|| invalid("Select bounds overflow"))?;
    if !(1..=32768).contains(&width) || !(1..=32768).contains(&height) {
        return Err(invalid("Invalid select bounds"));
    }
    Ok((width, height))
}
impl Control {
    /// 隐藏创建；无 CBS_SORT，系统索引始终对应声明顺序，包括重复值/标签。
    pub fn new(parent: HWND, rect: RECT, options: Options) -> Result<Self> {
        let (width, height) = size(rect)?;
        unsafe {
            if !IsWindow(Some(parent)).as_bool()
                || GetWindowThreadProcessId(parent, None) != GetCurrentThreadId()
            {
                return Err(invalid("Select parent must belong to current thread"));
            }
            items::install(parent)?;
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("COMBOBOX"),
                w!(""),
                WS_CHILD
                    | WS_TABSTOP
                    | WS_VSCROLL
                    | WINDOW_STYLE(
                        CBS_DROPDOWNLIST as u32 | CBS_HASSTRINGS as u32 | CBS_OWNERDRAWFIXED as u32,
                    ),
                rect.left,
                rect.top,
                width,
                height,
                Some(parent),
                None,
                None,
                None,
            )?;
            let marker = Box::new(0_u8);
            if let Err(error) = SetPropW(
                hwnd,
                OWNER,
                Some(HANDLE((&*marker as *const u8).cast_mut().cast())),
            ) {
                let _ = DestroyWindow(hwnd);
                return Err(error);
            }
            let owner = Self {
                hwnd,
                marker,
                options,
                _thread: PhantomData,
            };
            messages::install(hwnd)?;
            SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(GetStockObject(DEFAULT_GUI_FONT).0 as usize)),
                None,
            );
            items::resize(hwnd)?;
            for (index, item) in owner.options.items().iter().enumerate() {
                let text = HSTRING::from(item.label());
                let actual = SendMessageW(
                    owner.hwnd()?,
                    CB_ADDSTRING,
                    None,
                    Some(LPARAM(text.as_ptr() as isize)),
                )
                .0;
                if actual != index as isize {
                    return Err(Error::new(
                        E_FAIL,
                        "Select option insertion failed or reordered",
                    ));
                }
            }
            owner.select(owner.options.source_index())?;
            Ok(owner)
        }
    }
    pub fn is_alive(&self) -> bool {
        unsafe {
            IsWindow(Some(self.hwnd)).as_bool()
                && GetPropW(self.hwnd, OWNER).0 == (&*self.marker as *const u8).cast_mut().cast()
        }
    }
    pub fn hwnd(&self) -> Result<HWND> {
        if self.is_alive() {
            Ok(self.hwnd)
        } else {
            Err(Error::new(E_FAIL, "Select control closed"))
        }
    }
    pub fn selected_index(&self) -> Result<Option<usize>> {
        let index = unsafe { SendMessageW(self.hwnd()?, CB_GETCURSEL, None, None).0 };
        self.hwnd()?;
        if index == CB_ERR as isize {
            return Ok(None);
        }
        if index < 0 || index as usize >= self.options.items().len() {
            return Err(invalid("Select returned invalid option index"));
        }
        Ok(Some(index as usize))
    }
    pub fn value(&self) -> Result<Option<&str>> {
        Ok(self
            .selected_index()?
            .and_then(|index| self.options.value_at(index)))
    }
    /// 声明同步，不伪造 CBN_SELCHANGE；业务层只处理真实控件通知。
    pub fn select(&self, index: Option<usize>) -> Result<()> {
        if index.is_some_and(|index| index >= self.options.items().len()) {
            return Err(invalid("Select index exceeds option list"));
        }
        let expected = index.map(|index| index as isize).unwrap_or(CB_ERR as isize);
        let actual = unsafe {
            SendMessageW(
                self.hwnd()?,
                CB_SETCURSEL,
                Some(WPARAM(expected as usize)),
                None,
            )
            .0
        };
        self.hwnd()?;
        if actual != expected {
            return Err(Error::new(E_FAIL, "Select index update failed"));
        }
        Ok(())
    }
    pub fn show(&self) -> Result<()> {
        unsafe {
            let _ = ShowWindow(self.hwnd()?, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
    pub fn place(&self, rect: RECT) -> Result<()> {
        let (width, height) = size(rect)?;
        unsafe {
            SetWindowPos(
                self.hwnd()?,
                None,
                rect.left,
                rect.top,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
        }
    }
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        let hwnd = self.hwnd()?;
        unsafe {
            if !enabled {
                SendMessageW(hwnd, CB_SHOWDROPDOWN, Some(WPARAM(0)), None);
            }
            let _ = EnableWindow(self.hwnd()?, enabled);
            if IsWindowEnabled(self.hwnd()?).as_bool() != enabled {
                return Err(Error::new(E_FAIL, "Select enabled state mismatch"));
            }
        }
        Ok(())
    }
    pub fn close_popup(&self) -> Result<()> {
        unsafe {
            SendMessageW(self.hwnd()?, CB_SHOWDROPDOWN, Some(WPARAM(0)), None);
        }
        self.hwnd()?;
        Ok(())
    }
    pub fn set_dpi(&self, dpi: u32) -> Result<()> {
        font::update(self.hwnd()?, dpi, None)?;
        items::resize(self.hwnd()?)?;
        self.hwnd()?;
        Ok(())
    }
    pub fn set_color(&self, value: Option<crate::surface_color::Color>) -> Result<()> {
        color::update(self.hwnd()?, value)
    }
    pub fn set_font(
        &self,
        dpi: u32,
        style: &crate::surface_presentation::ControlFont,
    ) -> Result<()> {
        font::update(self.hwnd()?, dpi, Some(style))?;
        items::resize(self.hwnd()?)?;
        self.hwnd()?;
        Ok(())
    }
    pub fn focus(&self) -> Result<()> {
        unsafe {
            let hwnd = self.hwnd()?;
            if !IsWindowEnabled(hwnd).as_bool() {
                return Err(invalid("Select is disabled"));
            }
            let _ = SetFocus(Some(hwnd));
            if self.hwnd()? != hwnd || GetFocus() != hwnd {
                return Err(Error::new(E_FAIL, "Select focus not acquired"));
            }
        }
        Ok(())
    }
}
impl Drop for Control {
    fn drop(&mut self) {
        if self.is_alive() {
            unsafe {
                let _ = RemovePropW(self.hwnd, OWNER);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
#[cfg(test)]
#[path = "surface_select_control_tests.rs"]
mod tests;
