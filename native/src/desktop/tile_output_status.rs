//! 输出缺席时唯一可停止的短状态窗；不抢焦点、不注册hook/快捷键，关闭是永久停止意图。
use super::*;
use std::{cell::Cell, sync::OnceLock};
use windows::{
    core::{w, HRESULT, PCWSTR},
    Win32::System::LibraryLoader::GetModuleHandleW,
};
struct State {
    closed: Cell<bool>,
}
pub(super) struct StatusWindow {
    hwnd: HWND,
    state: Box<State>,
    label: Cell<&'static str>,
}
impl StatusWindow {
    pub fn new() -> Result<Self> {
        unsafe {
            register()?;
            let state = Box::new(State {
                closed: Cell::new(false),
            });
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("Otter.Native.TileStatus"),
                w!("Otter Tile"),
                WS_CAPTION | WS_SYSMENU,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                320,
                120,
                None,
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some((&*state as *const State).cast()),
            )?;
            let window = Self {
                hwnd,
                state,
                label: Cell::new(""),
            };
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("停止输出"),
                WS_CHILD | WS_VISIBLE,
                85,
                20,
                140,
                32,
                Some(hwnd),
                Some(HMENU(std::ptr::dangling_mut())),
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                None,
            )?;
            Ok(window)
        }
    }
    pub fn stopped(&self) -> bool {
        self.state.closed.get()
    }
    pub fn show(&self, label: &'static str) {
        if self.stopped() {
            return;
        }
        if self.label.replace(label) != label {
            let wide: Vec<_> = label.encode_utf16().chain(Some(0)).collect();
            unsafe {
                let _ = SetWindowTextW(self.hwnd, PCWSTR(wide.as_ptr()));
            }
        }
        // SetWindowText/ShowWindow都可能同步重入；关闭后不再写旧HWND。
        if !self.stopped() && !unsafe { IsWindowVisible(self.hwnd).as_bool() } {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            }
        }
    }
    pub fn hide(&self) {
        if !self.stopped() && unsafe { IsWindowVisible(self.hwnd).as_bool() } {
            unsafe {
                let _ = ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }
}
impl Drop for StatusWindow {
    fn drop(&mut self) {
        if !self.stopped() {
            unsafe {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
unsafe fn register() -> Result<()> {
    static ONCE: OnceLock<std::result::Result<(), HRESULT>> = OnceLock::new();
    ONCE.get_or_init(|| {
        (|| -> Result<()> {
            if RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(procedure),
                hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                lpszClassName: w!("Otter.Native.TileStatus"),
                ..Default::default()
            }) == 0
            {
                Err(Error::from_win32())
            } else {
                Ok(())
            }
        })()
        .map_err(|e| e.code())
    })
    .map_err(Error::from_hresult)
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (*(lp.0 as *const CREATESTRUCTW)).lpCreateParams as isize,
        );
    }
    let Some(state) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref() else {
        return DefWindowProcW(hwnd, msg, wp, lp);
    };
    match msg {
        WM_COMMAND if wp.0 & 0xffff == 1 => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = DestroyWindow(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => {
            state.closed.set(true);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}
#[cfg(test)]
#[path = "tile_output_status_tests.rs"]
mod tests;
