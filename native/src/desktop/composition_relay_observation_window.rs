//! Viewer所属只读文本窗；无网络/定时器/线程，关闭或撤权时销毁敏感文本。
use crate::live_relay_viewer_business::Business;
use std::{
    cell::Cell,
    sync::{Arc, OnceLock},
};
use windows::{
    core::{w, Error, Result, HRESULT, HSTRING},
    Win32::{
        Foundation::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Controls::{
                EM_GETFIRSTVISIBLELINE, EM_GETSEL, EM_LINESCROLL, EM_SETLIMITTEXT, EM_SETSEL,
            },
            WindowsAndMessaging::*,
        },
    },
};
#[path = "composition_relay_observation_text.rs"]
mod text;
struct State {
    edit: Cell<HWND>,
}
pub(crate) struct Window {
    hwnd: HWND,
    state: Box<State>,
    shown: Option<Arc<Business>>,
    initialized: bool,
}
impl Window {
    pub(super) fn new(owner: HWND) -> Result<Self> {
        unsafe {
            register()?;
            let state = Box::new(State {
                edit: Cell::new(HWND::default()),
            });
            let mut rect = RECT::default();
            GetWindowRect(owner, &mut rect)?;
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("Otter.Native.LiveObservations"),
                w!("Otter · 观察与触发记录"),
                WS_OVERLAPPEDWINDOW,
                rect.left.saturating_add(24),
                rect.top.saturating_add(24),
                720,
                560,
                Some(owner),
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some((&*state as *const State).cast()),
            )?;
            let window = Self {
                hwnd,
                state,
                shown: None,
                initialized: false,
            };
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
            let edit = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                w!("EDIT"),
                None,
                WS_CHILD
                    | WS_VISIBLE
                    | WS_VSCROLL
                    | WS_HSCROLL
                    | WINDOW_STYLE(
                        (ES_MULTILINE | ES_READONLY | ES_AUTOVSCROLL | ES_AUTOHSCROLL) as u32,
                    ),
                0,
                0,
                0,
                0,
                Some(hwnd),
                None,
                None,
                None,
            )?;
            window.state.edit.set(edit);
            let font = windows::Win32::Graphics::Gdi::GetStockObject(
                windows::Win32::Graphics::Gdi::DEFAULT_GUI_FONT,
            );
            SendMessageW(
                edit,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(0)),
            );
            SendMessageW(
                edit,
                EM_SETLIMITTEXT,
                Some(WPARAM(text::MAX_TEXT)),
                Some(LPARAM(0)),
            );
            layout(hwnd, &window.state)?;
            Ok(window)
        }
    }
    pub(super) fn handle(&self) -> Option<HWND> {
        unsafe {
            (IsWindow(Some(self.hwnd)).as_bool()
                && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA)
                    == &*self.state as *const State as isize)
                .then_some(self.hwnd)
        }
    }
    pub(super) fn update(&mut self, business: Option<Arc<Business>>) -> Result<()> {
        if self.handle().is_none() {
            return Ok(());
        }
        if self.initialized
            && match (&self.shown, &business) {
                (Some(a), Some(b)) => a.same_display(b),
                (None, None) => true,
                _ => false,
            }
        {
            return Ok(());
        }
        let value = HSTRING::from(text::render(business.as_deref()));
        // 标准EDIT同步发送通知；不持Relay/Host/面板的RefCell借用跨重入。
        unsafe {
            let edit = self.state.edit.get();
            let mut start = 0u32;
            let mut end = 0u32;
            SendMessageW(
                edit,
                EM_GETSEL,
                Some(WPARAM(&mut start as *mut _ as usize)),
                Some(LPARAM(&mut end as *mut _ as isize)),
            );
            let first = SendMessageW(
                edit,
                EM_GETFIRSTVISIBLELINE,
                Some(WPARAM(0)),
                Some(LPARAM(0)),
            )
            .0;
            SetWindowTextW(edit, &value)?;
            // SETTEXT可同步销毁owner；确认同一窗口后才恢复选区/阅读位置，不写可能复用的HWND。
            if self.handle().is_none() {
                return Ok(());
            }
            SendMessageW(
                edit,
                EM_SETSEL,
                Some(WPARAM(start as usize)),
                Some(LPARAM(end as isize)),
            );
            let current = SendMessageW(
                edit,
                EM_GETFIRSTVISIBLELINE,
                Some(WPARAM(0)),
                Some(LPARAM(0)),
            )
            .0;
            SendMessageW(
                edit,
                EM_LINESCROLL,
                Some(WPARAM(0)),
                Some(LPARAM(first.saturating_sub(current))),
            );
        }
        if self.handle().is_some() {
            self.shown = business;
            self.initialized = true;
        }
        Ok(())
    }
    pub(super) fn show(&self) {
        if let Some(hwnd) = self.handle() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            }
        }
    }
    #[cfg(test)]
    pub(super) fn edit(&self) -> HWND {
        self.state.edit.get()
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if self.handle().is_some() {
                let _ = DestroyWindow(self.hwnd);
            }
        }
    }
}
unsafe fn layout(hwnd: HWND, state: &State) -> Result<()> {
    if state.edit.get().is_invalid() {
        return Ok(());
    }
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect)?;
    MoveWindow(
        state.edit.get(),
        0,
        0,
        (rect.right - rect.left).max(0),
        (rect.bottom - rect.top).max(0),
        true,
    )
}
unsafe fn register() -> Result<()> {
    static REGISTERED: OnceLock<std::result::Result<(), HRESULT>> = OnceLock::new();
    REGISTERED
        .get_or_init(|| {
            (|| -> Result<()> {
                if RegisterClassW(&WNDCLASSW {
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                    hCursor: LoadCursorW(None, IDC_ARROW)?,
                    lpszClassName: w!("Otter.Native.LiveObservations"),
                    ..Default::default()
                }) == 0
                {
                    return Err(Error::from_win32());
                }
                Ok(())
            })()
            .map_err(|e| e.code())
        })
        .map_err(Error::from_hresult)
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    if message == WM_NCDESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
    }
    if message == WM_SIZE {
        if let Some(state) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State).as_ref() {
            if layout(hwnd, state).is_err() {
                let _ = DestroyWindow(hwnd);
            }
            return LRESULT(0);
        }
    }
    DefWindowProcW(hwnd, message, wp, lp)
}
