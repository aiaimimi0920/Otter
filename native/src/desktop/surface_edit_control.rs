//! 单线程原生 EDIT owner；仅提供有界编辑与资源生命周期，Surface 事件/草稿由上层拥有。
use std::{marker::PhantomData, rc::Rc};
use windows::Win32::UI::Controls::EM_SETLIMITTEXT;
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::Threading::GetCurrentThreadId,
        UI::{
            Input::KeyboardAndMouse::{GetFocus, SetFocus},
            WindowsAndMessaging::*,
        },
    },
};

pub const MAX_UNITS: usize = 64 * 1024;
pub const MAX_BYTES: usize = crate::surface_input_draft::MAX_VALUE_BYTES;
#[path = "surface_edit_alignment.rs"]
mod alignment;
#[path = "surface_edit_colors.rs"]
mod colors;
pub(super) use colors::paint;
#[path = "surface_edit_font.rs"]
mod font;
#[path = "surface_edit_placeholder.rs"]
mod placeholder;
const OWNER: windows::core::PCWSTR = w!("Otter.Surface.Edit.Owner");
pub struct Control {
    hwnd: HWND,
    marker: Box<u8>,
    limit: usize,
    multiline: bool,
    _thread: PhantomData<Rc<()>>,
}
fn invalid(message: &str) -> Error {
    Error::new(E_INVALIDARG, message)
}
fn size(rect: RECT) -> Result<(i32, i32)> {
    let width = rect
        .right
        .checked_sub(rect.left)
        .ok_or_else(|| invalid("Edit bounds overflow"))?;
    let height = rect
        .bottom
        .checked_sub(rect.top)
        .ok_or_else(|| invalid("Edit bounds overflow"))?;
    if !(1..=32768).contains(&width) || !(1..=32768).contains(&height) {
        return Err(invalid("Invalid edit bounds"));
    }
    Ok((width, height))
}
impl Control {
    /// 隐藏创建，调用方在绑定通知/草稿身份之后才显示，不能跨 HWND 线程使用。
    pub fn new(parent: HWND, rect: RECT, multiline: bool, limit: usize) -> Result<Self> {
        let (width, height) = size(rect)?;
        if !(1..=MAX_UNITS).contains(&limit) {
            return Err(invalid("Invalid edit text limit"));
        }
        unsafe {
            if !IsWindow(Some(parent)).as_bool()
                || GetWindowThreadProcessId(parent, None) != GetCurrentThreadId()
            {
                return Err(invalid("Edit parent must belong to current thread"));
            }
            let behavior = if multiline {
                ES_MULTILINE | ES_AUTOVSCROLL | ES_WANTRETURN
            } else {
                ES_AUTOHSCROLL
            };
            let style = WS_CHILD
                | WS_TABSTOP
                | WINDOW_STYLE(behavior as u32)
                | if multiline {
                    WS_VSCROLL
                } else {
                    WINDOW_STYLE(0)
                };
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("EDIT"),
                w!(""),
                style,
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
                limit,
                multiline,
                _thread: PhantomData,
            };
            SendMessageW(hwnd, EM_SETLIMITTEXT, Some(WPARAM(limit)), None);
            // 共享系统字体不归本 owner 销毁；后续样式字体必须另设显式 owner。
            SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(GetStockObject(DEFAULT_GUI_FONT).0 as usize)),
                None,
            );
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
            Err(Error::new(E_FAIL, "Edit control closed"))
        }
    }
    pub fn show(&self) -> Result<()> {
        unsafe {
            // 返回的是此前可见性，不是显示操作的成功标志。
            let _ = ShowWindow(self.hwnd()?, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
    pub fn install_placeholder(&self, text: &str) -> Result<()> {
        placeholder::install(self.hwnd()?, text, self.multiline)
    }
    pub fn set_font(
        &self,
        dpi: u32,
        style: &crate::surface_presentation::ControlFont,
    ) -> Result<()> {
        font::update(self.hwnd()?, dpi, style)?;
        self.hwnd()?;
        Ok(())
    }
    pub fn selection(&self) -> Result<(u32, u32)> {
        let mut start = 0_u32;
        let mut end = 0_u32;
        unsafe {
            SendMessageW(
                self.hwnd()?,
                windows::Win32::UI::Controls::EM_GETSEL,
                Some(WPARAM((&mut start as *mut u32) as usize)),
                Some(LPARAM((&mut end as *mut u32) as isize)),
            );
        }
        Ok((start, end))
    }
    pub fn select(&self, selection: (u32, u32)) -> Result<()> {
        unsafe {
            SendMessageW(
                self.hwnd()?,
                windows::Win32::UI::Controls::EM_SETSEL,
                Some(WPARAM(selection.0 as usize)),
                Some(LPARAM(selection.1 as isize)),
            );
        }
        Ok(())
    }
    pub fn focus(&self) -> Result<()> {
        unsafe {
            let hwnd = self.hwnd()?;
            // 返回值是先前焦点 HWND；原本无焦点时 NULL 不代表聚焦失败。
            let _ = SetFocus(Some(hwnd));
            if self.hwnd()? != hwnd || GetFocus() != hwnd {
                return Err(Error::new(E_FAIL, "Edit focus not acquired"));
            }
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
    /// WM_SETTEXT 不受 EM_SETLIMITTEXT 保护，写入前独立验证两个容量和 NUL 边界。
    pub fn set_text(&self, text: &str) -> Result<()> {
        self.validate_text(text)?;
        unsafe { SetWindowTextW(self.hwnd()?, &HSTRING::from(text)) }
    }
    pub fn validate_text(&self, text: &str) -> Result<()> {
        if text.len() > MAX_BYTES
            || text.encode_utf16().count() > self.limit
            || text.contains('\0')
            || (!self.multiline && text.contains(['\r', '\n']))
        {
            return Err(invalid("Invalid or oversized edit text"));
        }
        Ok(())
    }
    /// 不以外部 HWND 长度决定分配；超限/畸形 UTF-16 返回错误，不静默截断用户输入。
    pub fn text(&self) -> Result<String> {
        let hwnd = self.hwnd()?;
        unsafe {
            let length = GetWindowTextLengthW(hwnd);
            if length < 0 || length as usize > self.limit {
                return Err(invalid("Edit text exceeds unit limit"));
            }
            // 长度已先验证上限；普通短文本不为每个按键分配完整 64K 缓冲。
            let mut buffer = vec![0_u16; length as usize + 1];
            let count = GetWindowTextW(hwnd, &mut buffer) as usize;
            if count != length as usize || !self.is_alive() || GetWindowTextLengthW(hwnd) != length
            {
                return Err(Error::new(E_FAIL, "Edit text changed during read"));
            }
            let value =
                String::from_utf16(&buffer[..count]).map_err(|_| invalid("Invalid edit UTF-16"))?;
            if value.len() > MAX_BYTES {
                return Err(invalid("Edit text exceeds byte limit"));
            }
            Ok(value)
        }
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
#[path = "surface_edit_control_tests.rs"]
mod tests;
