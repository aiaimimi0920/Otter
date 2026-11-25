//! Application-level modeless history window; standard buttons supply accessible names/keyboard behavior, while business logic enqueues only single-slot intents.
use crate::desktop::capture_history_ui::{Action, Origin, Shared};
use std::{
    cell::{Cell, RefCell},
    sync::{Arc, OnceLock},
};
use windows::{
    core::{w, Error, Result, HRESULT, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{Controls::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};
#[path = "capture_history_window_draw.rs"]
mod draw;
#[path = "capture_history_window_layout.rs"]
mod layout;
struct State {
    view: Shared,
    origin: Origin,
    data: RefCell<Arc<crate::capture_history::Data>>,
    status: RefCell<String>,
    language: Cell<crate::ui_text::DisplayLanguage>,
    scroll: Cell<i32>,
    children: RefCell<Vec<HWND>>,
}
pub(in crate::desktop::application) struct Window {
    hwnd: HWND,
    state: Box<State>,
    revision: u64,
    restore: HWND,
}
impl Window {
    pub fn new(source: HWND, origin: Origin, view: Shared) -> Result<Self> {
        unsafe {
            register()?;
            let state = Box::new(State {
                view,
                origin,
                data: Default::default(),
                status: Default::default(),
                language: Cell::new(crate::ui_text::current_language()),
                scroll: Cell::new(0),
                children: Default::default(),
            });
            let mut rect = RECT::default();
            GetWindowRect(source, &mut rect)?;
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("Otter.Native.CaptureHistory"),
                &HSTRING::from(crate::ui_text::text("capture-history-title")),
                WS_OVERLAPPEDWINDOW | WS_VSCROLL | WS_CLIPCHILDREN,
                rect.left.saturating_add(24),
                rect.top.saturating_add(24),
                470,
                650,
                Some(source),
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some((&*state as *const State).cast()),
            )?;
            let mut window = Self {
                hwnd,
                state,
                revision: u64::MAX,
                restore: source,
            };
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
            layout::create(hwnd, &window.state)?;
            window.update()?;
            Ok(window)
        }
    }
    pub fn handle(&self) -> Option<HWND> {
        unsafe {
            (IsWindow(Some(self.hwnd)).as_bool()
                && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA)
                    == &*self.state as *const State as isize)
                .then_some(self.hwnd)
        }
    }
    pub fn show(&self) {
        if let Some(hwnd) = self.handle() {
            unsafe {
                let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
            }
        }
    }
    pub fn update(&mut self) -> Result<()> {
        let Some(hwnd) = self.handle() else {
            return Ok(());
        };
        self.refresh_language(crate::ui_text::current_language())?;
        unsafe {
            // Do not change entry identity while a button is pressed; the original message pump updates again after release/cancellation.
            if IsChild(hwnd, GetCapture()).as_bool()
                || GetKeyState(VK_SPACE.0 as i32) < 0
                || GetKeyState(VK_RETURN.0 as i32) < 0
            {
                return Ok(());
            }
        }
        let view = self.state.view.borrow();
        let data_changed = self.revision != view.revision;
        if data_changed {
            self.revision = view.revision;
            *self.state.data.borrow_mut() = view.data.clone();
        }
        let status = view.status.as_deref().unwrap_or_default();
        let status_changed = self.state.status.borrow().as_str() != status;
        if status_changed {
            status.clone_into(&mut self.state.status.borrow_mut());
        }
        drop(view);
        // Status text is painted by the parent; it never changes button geometry or identity.
        unsafe {
            if data_changed {
                layout::update(hwnd, &self.state)?;
            } else if status_changed {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
        }
        Ok(())
    }
    fn refresh_language(&self, language: crate::ui_text::DisplayLanguage) -> Result<()> {
        if self.state.language.get() == language {
            return Ok(());
        }
        let Some(hwnd) = self.handle() else {
            return Ok(());
        };
        unsafe {
            SetWindowTextW(
                hwnd,
                &HSTRING::from(crate::ui_text::text_for(language, "capture-history-title")),
            )?;
            // Presentation never replaces entries, moves controls, or clamps the current scroll.
            layout::labels(&self.state, language)?;
            self.state.language.set(language);
            let _ = RedrawWindow(Some(hwnd), None, None, RDW_INVALIDATE | RDW_ALLCHILDREN);
        }
        Ok(())
    }
    pub fn translate(&self, message: &MSG) -> bool {
        let Some(hwnd) = self.handle() else {
            return false;
        };
        unsafe {
            if message.hwnd != hwnd && !IsChild(hwnd, message.hwnd).as_bool() {
                return false;
            }
            if message.message == WM_KEYDOWN {
                if message.wParam.0 == VK_ESCAPE.0 as usize
                    || (message.wParam.0 == 0x48 && GetKeyState(VK_CONTROL.0 as i32) < 0)
                {
                    let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
                    return true;
                }
                if message.wParam.0 == VK_RETURN.0 as usize && message.hwnd != hwnd {
                    if message.lParam.0 & (1 << 30) == 0 {
                        SendMessageW(message.hwnd, BM_CLICK, None, None);
                    }
                    return true;
                }
            }
            IsDialogMessageW(hwnd, message).as_bool()
        }
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if let Some(hwnd) = self.handle() {
                let foreground = GetForegroundWindow();
                let restore = foreground == hwnd || IsChild(hwnd, foreground).as_bool();
                let _ = DestroyWindow(hwnd);
                if restore && IsWindow(Some(self.restore)).as_bool() {
                    let _ = SetForegroundWindow(self.restore);
                }
            }
        }
    }
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
                    lpszClassName: w!("Otter.Native.CaptureHistory"),
                    ..Default::default()
                }) == 0
                {
                    return Err(Error::from_win32());
                }
                Ok(())
            })()
            .map_err(|error| error.code())
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
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        }
        WM_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        WM_DPICHANGED => {
            let rect = &*(lp.0 as *const RECT);
            let _ = SetWindowPos(
                hwnd,
                None,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            let _ = layout::resize(hwnd, state);
        }
        WM_SIZE => {
            let _ = layout::resize(hwnd, state);
        }
        WM_VSCROLL | WM_MOUSEWHEEL => {
            layout::scroll(hwnd, state, msg, wp);
        }
        WM_DRAWITEM => {
            draw::item(state, &*(lp.0 as *const DRAWITEMSTRUCT));
        }
        WM_PAINT => {
            draw::paint(hwnd, state);
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_COMMAND => {
            let id = wp.0 & 0xffff;
            let code = (wp.0 >> 16) as u32;
            let child = HWND(lp.0 as *mut _);
            if !state.children.borrow().contains(&child) || GetDlgCtrlID(child) as usize != id {
                return LRESULT(0);
            }
            if code == BN_SETFOCUS {
                layout::focus(hwnd, state, HWND(lp.0 as *mut _));
            }
            if code == BN_CLICKED {
                let action = {
                    let data = state.data.borrow();
                    match id {
                        100..=123 => data.colors.get(id - 100).map(|entry| {
                            Action::Color(
                                crate::stroke_style::Color(entry.rgb.r, entry.rgb.g, entry.rgb.b),
                                entry.hex.clone(),
                            )
                        }),
                        200..=223 => data
                            .screenshots
                            .get(id - 200)
                            .map(|entry| Action::Reuse(entry.id.clone())),
                        300..=323 => data
                            .screenshots
                            .get(id - 300)
                            .map(|entry| Action::Delete(entry.id.clone())),
                        _ => None,
                    }
                };
                if let Some(action) = action {
                    state.view.borrow_mut().action = Some((state.origin.clone(), action));
                }
            }
        }
        _ => return DefWindowProcW(hwnd, msg, wp, lp),
    }
    if msg == WM_NCDESTROY {
        DefWindowProcW(hwnd, msg, wp, lp)
    } else {
        LRESULT(0)
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "capture_history_window_tests.rs"]
mod tests;
