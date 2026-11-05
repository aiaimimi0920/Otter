//! Number 的系统步进按钮 owner；不允许系统控件回写整数，浮点草稿仍由 Session 拥有。
use std::{marker::PhantomData, rc::Rc};
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        System::Threading::GetCurrentThreadId,
        UI::{Controls::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
const OWNER: windows::core::PCWSTR = w!("Otter.Surface.Number.Spinner.Owner");
pub struct Spinner {
    hwnd: HWND,
    marker: Box<u8>,
    _thread: PhantomData<Rc<()>>,
}
impl Spinner {
    pub fn new(parent: HWND) -> Result<Self> {
        unsafe {
            if !IsWindow(Some(parent)).as_bool()
                || GetWindowThreadProcessId(parent, None) != GetCurrentThreadId()
            {
                return Err(Error::new(
                    E_INVALIDARG,
                    "Spinner parent must belong to current thread",
                ));
            }
            InitCommonControlsEx(&INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_UPDOWN_CLASS,
            })
            .ok()?;
            // 无 buddy、SETBUDDYINT、ARROWKEYS；系统只拥有命中、捕获和重复计时。
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                UPDOWN_CLASSW,
                w!(""),
                WS_CHILD | WS_DISABLED | WINDOW_STYLE(UDS_HOTTRACK),
                0,
                0,
                1,
                1,
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
                _thread: PhantomData,
            };
            SendMessageW(
                hwnd,
                UDM_SETRANGE32,
                Some(WPARAM((-1_i32) as usize)),
                Some(LPARAM(1)),
            );
            SendMessageW(hwnd, UDM_SETPOS32, None, Some(LPARAM(0)));
            let acceleration = UDACCEL { nSec: 0, nInc: 1 };
            SendMessageW(
                hwnd,
                UDM_SETACCEL,
                Some(WPARAM(1)),
                Some(LPARAM((&acceleration as *const UDACCEL) as isize)),
            );
            Ok(owner)
        }
    }
    pub fn owns(&self, hwnd: HWND) -> bool {
        hwnd == self.hwnd
            && unsafe {
                IsWindow(Some(hwnd)).as_bool()
                    && GetPropW(hwnd, OWNER).0 == (&*self.marker as *const u8).cast_mut().cast()
            }
    }
    pub fn hwnd(&self) -> Result<HWND> {
        self.owns(self.hwnd)
            .then_some(self.hwnd)
            .ok_or_else(|| Error::new(E_FAIL, "Spinner closed"))
    }
    pub fn cancel(&self) {
        if self.owns(self.hwnd) {
            unsafe {
                SendMessageW(self.hwnd, WM_CANCELMODE, None, None);
                if GetCapture() == self.hwnd {
                    let _ = ReleaseCapture();
                }
            }
        }
    }
    pub fn enable(&self, enabled: bool, visible: bool) -> Result<()> {
        let hwnd = self.hwnd()?;
        if !enabled || !visible {
            self.cancel();
        }
        unsafe {
            let _ = EnableWindow(hwnd, enabled && visible);
            let _ = ShowWindow(hwnd, if visible { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
        Ok(())
    }
}
impl Drop for Spinner {
    fn drop(&mut self) {
        if self.owns(self.hwnd) {
            self.cancel();
            unsafe {
                let _ = RemovePropW(self.hwnd, OWNER);
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
