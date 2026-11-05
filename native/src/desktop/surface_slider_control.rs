//! 原生自绘 Slider 子窗口 owner；浮点值不映射为低精度整数刻度，不拥有业务提交权。
use crate::surface_slider::Range;
use std::{marker::PhantomData, rc::Rc};
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::Threading::GetCurrentThreadId,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
#[path = "surface_slider_messages.rs"]
mod messages;
#[path = "surface_slider_paint.rs"]
mod paint;
const OWNER: windows::core::PCWSTR = w!("Otter.Surface.Slider.Owner");
const STATE: windows::core::PCWSTR = w!("Otter.Surface.Slider.State");
/// 私有同线程通知：wp=0 input、wp=1 change；lp=实际 HWND。接收者仍须验证挂载身份。
pub const NOTIFY: u32 = WM_APP + 0x55d;
pub const CLICK_BEGIN: u32 = WM_APP + 0x55e;
pub const CLICK_CANCEL: u32 = WM_APP + 0x55f;
pub const CLICK_END: u32 = WM_APP + 0x560;
struct State {
    range: Range,
    value: f64,
    drag: Option<f64>,
    drag_touch: bool,
    click_active: bool,
}
unsafe fn state(hwnd: HWND) -> *mut State {
    GetPropW(hwnd, STATE).0.cast()
}
unsafe fn alive(hwnd: HWND) -> bool {
    IsWindow(Some(hwnd)).as_bool() && !GetPropW(hwnd, OWNER).is_invalid() && !state(hwnd).is_null()
}
unsafe fn take_state(hwnd: HWND) {
    if let Ok(value) = RemovePropW(hwnd, STATE) {
        if !value.is_invalid() {
            drop(Box::from_raw(value.0.cast::<State>()));
        }
    }
}
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid or closed Slider")
}
fn dimensions(rect: RECT) -> Result<(i32, i32)> {
    let width = rect.right.checked_sub(rect.left).ok_or_else(invalid)?;
    let height = rect.bottom.checked_sub(rect.top).ok_or_else(invalid)?;
    if !(1..=32768).contains(&width) || !(1..=32768).contains(&height) {
        return Err(invalid());
    }
    Ok((width, height))
}
pub struct Control {
    hwnd: HWND,
    marker: Box<u8>,
    _thread: PhantomData<Rc<()>>,
}
impl Control {
    pub fn new(parent: HWND, rect: RECT, range: Range) -> Result<Self> {
        let (width, height) = dimensions(rect)?;
        unsafe {
            if !IsWindow(Some(parent)).as_bool()
                || GetWindowThreadProcessId(parent, None) != GetCurrentThreadId()
            {
                return Err(invalid());
            }
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!(""),
                WS_CHILD
                    | WS_TABSTOP
                    | WINDOW_STYLE(windows::Win32::System::SystemServices::SS_NOTIFY.0),
                rect.left,
                rect.top,
                width,
                height,
                Some(parent),
                None,
                None,
                None,
            )?;
            let marker = Box::new(0u8);
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
                _thread: PhantomData,
            };
            let mut data = Box::new(State {
                range,
                value: range.source(),
                drag: None,
                drag_touch: false,
                click_active: false,
            });
            SetPropW(hwnd, STATE, Some(HANDLE((&mut *data as *mut State).cast())))?;
            let _ = Box::into_raw(data);
            if let Err(error) = messages::install(hwnd) {
                take_state(hwnd);
                return Err(error);
            }
            Ok(owner)
        }
    }
    fn owns(&self) -> bool {
        unsafe {
            IsWindow(Some(self.hwnd)).as_bool()
                && GetPropW(self.hwnd, OWNER).0 == (&*self.marker as *const u8).cast_mut().cast()
        }
    }
    pub fn hwnd(&self) -> Result<HWND> {
        if self.owns() && unsafe { !state(self.hwnd).is_null() } {
            Ok(self.hwnd)
        } else {
            Err(invalid())
        }
    }
    pub fn value(&self) -> Result<f64> {
        unsafe { Ok((*state(self.hwnd()?)).value) }
    }
    pub(crate) fn range(&self) -> Result<Range> {
        unsafe { Ok((*state(self.hwnd()?)).range) }
    }
    pub fn set_value(&self, value: f64) -> Result<()> {
        let hwnd = self.hwnd()?;
        unsafe {
            let next = (*state(hwnd))
                .range
                .normalize(value)
                .map_err(|_| invalid())?;
            messages::cancel(hwnd);
            (*state(self.hwnd()?)).value = next;
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        self.hwnd()?;
        Ok(())
    }
    pub fn show(&self) -> Result<()> {
        unsafe {
            let _ = ShowWindow(self.hwnd()?, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
    pub fn cancel(&self) -> Result<()> {
        unsafe {
            messages::cancel(self.hwnd()?);
        }
        self.hwnd()?;
        Ok(())
    }
    pub fn focus(&self) -> Result<()> {
        unsafe {
            let hwnd = self.hwnd()?;
            if !IsWindowEnabled(hwnd).as_bool() {
                return Err(invalid());
            }
            let _ = SetFocus(Some(hwnd));
            if self.hwnd()? != hwnd || GetFocus() != hwnd {
                return Err(invalid());
            }
        }
        Ok(())
    }
    pub fn place(&self, rect: RECT) -> Result<()> {
        let (width, height) = dimensions(rect)?;
        unsafe {
            messages::cancel(self.hwnd()?);
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
        unsafe {
            if !enabled {
                messages::cancel(self.hwnd()?);
            }
            let _ = EnableWindow(self.hwnd()?, enabled);
        }
        self.hwnd()?;
        Ok(())
    }
}
impl Drop for Control {
    fn drop(&mut self) {
        // 初始化失败时可能尚无 State，但已创建的自有窗口仍必须销毁。
        if self.owns() {
            unsafe {
                let _ = RemovePropW(self.hwnd, OWNER);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
#[cfg(test)]
#[path = "surface_slider_control_tests.rs"]
mod tests;
