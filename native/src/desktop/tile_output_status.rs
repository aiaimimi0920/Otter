//! The sole stoppable brief status window while output is absent; no focus stealing/hooks/shortcuts, and closing means permanent stop.
use super::*;
use std::{cell::Cell, sync::OnceLock};
use windows::{
    core::{w, HRESULT},
    Win32::System::LibraryLoader::GetModuleHandleW,
};
struct State {
    closed: Cell<bool>,
}
pub(super) struct StatusWindow {
    hwnd: HWND,
    state: Box<State>,
    label: Cell<Option<&'static str>>,
    stop_label: Cell<Option<&'static str>>,
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
                label: Cell::new(None),
                stop_label: Cell::new(None),
            };
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                &windows::core::HSTRING::from(crate::ui_text::text("integration-stop-output")),
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
        let stop_label = crate::ui_text::text("integration-stop-output");
        if self.stop_label.get() != Some(stop_label) {
            let updated = unsafe {
                GetDlgItem(Some(self.hwnd), 1).and_then(|button| {
                    SetWindowTextW(button, &windows::core::HSTRING::from(stop_label))
                })
            };
            if self.stopped() {
                return;
            }
            if updated.is_ok() {
                self.stop_label.set(Some(stop_label));
            }
        }
        if self.label.get() != Some(label) {
            // A failed write leaves the title unconfirmed. Let the next output
            // tick retry, but keep the stop button available in this tick.
            self.label.set(None);
            let wide: Vec<_> = label.encode_utf16().chain(Some(0)).collect();
            // Own-thread window: use the window procedure's acknowledgement,
            // not SetWindowTextW's wrapper status.
            let written = unsafe {
                SendMessageW(
                    self.hwnd,
                    WM_SETTEXT,
                    None,
                    Some(LPARAM(wide.as_ptr() as isize)),
                )
            };
            if written.0 == 1 && !self.stopped() {
                self.label.set(Some(label));
            }
        }
        // WM_SETTEXT/ShowWindow may synchronously reenter; never write the old HWND after closure.
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
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-surface")))]
#[path = "tile_output_status_tests.rs"]
mod tests;
