//! 远程观看 HWND；应用等待帧事件并调用 refresh，窗口回调不执行网络或线程 join。
use super::{Host, Update};
use crate::live_relay_viewer_session::{Permit, ViewerSession};
use std::{
    cell::{Cell, RefCell},
    os::windows::io::BorrowedHandle,
    sync::{Arc, OnceLock},
};
use windows::{
    core::{w, Error, Result, HRESULT},
    Win32::{
        Foundation::*,
        Graphics::{Direct2D::Common::D2D_SIZE_U, Gdi::*},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::*,
    },
};
#[path = "composition_relay_window_input.rs"]
pub(crate) mod input;
#[path = "composition_relay_window_local_trigger.rs"]
mod local_trigger;
#[path = "composition_relay_window_observation.rs"]
pub(crate) mod observation;
#[path = "composition_relay_window_trigger.rs"]
pub(crate) mod trigger;

struct State {
    session_end_reason: Cell<bool>,
    observation: RefCell<Option<observation::Window>>,
    input: RefCell<input::Control>,
    host: RefCell<Option<Host>>,
    error: RefCell<Option<Error>>,
    closed: Cell<bool>,
    deferred: Cell<bool>,
}
impl State {
    fn with_host<T>(&self, operation: impl FnOnce(&mut Host) -> Result<T>) -> Result<Option<T>> {
        let Some(mut host) = self.host.borrow_mut().take() else {
            self.deferred.set(true);
            return Ok(None);
        };
        let result = operation(&mut host);
        // D2D/Win32 可能重入并关闭窗口；不能把旧 Host 放回已终止状态。
        if !self.closed.get() {
            *self.host.borrow_mut() = Some(host);
        }
        result.map(Some)
    }
    fn stop(&self) {
        self.closed.set(true);
        let observation = self.observation.borrow_mut().take();
        drop(observation);
        self.input.borrow_mut().stop();
        let host = self.host.borrow_mut().take();
        drop(host);
    }
}
pub struct RelayWindow {
    hwnd: HWND,
    state: Box<State>,
    media: Arc<ViewerSession>,
    permit: Permit,
}
impl RelayWindow {
    #[cfg(test)]
    pub(super) fn test_frame(
        &self,
    ) -> Result<Option<(crate::live_relay_frame::Metadata, [u8; 4])>> {
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect)?;
        }
        self.state
            .with_host(|host| {
                host.test_frame(D2D_SIZE_U {
                    width: (rect.right - rect.left) as u32,
                    height: (rect.bottom - rect.top) as u32,
                })
            })
            .map(Option::flatten)
    }
    /// 创建隐藏窗口，启用系统捕获排除后绑定许可；显示必须由上层显式请求。
    pub fn new(media: Arc<ViewerSession>, permit: Permit, width: u32, height: u32) -> Result<Self> {
        super::super::validate_size(D2D_SIZE_U { width, height })?;
        unsafe {
            register()?;
            let state = Box::new(State {
                session_end_reason: Cell::new(false),
                observation: RefCell::new(None),
                input: RefCell::new(input::Control::default()),
                host: RefCell::new(None),
                error: RefCell::new(None),
                closed: Cell::new(false),
                deferred: Cell::new(false),
            });
            let hwnd = CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW,
                w!("Otter.Native.Relay"),
                w!("Otter 远程观看"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as i32,
                height as i32,
                None,
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some((&*state as *const State).cast()),
            )?;
            let window = Self {
                hwnd,
                state,
                media,
                permit: permit.clone(),
            };
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
            *window.state.host.borrow_mut() = Some(Host::bind(hwnd, window.media.clone(), permit)?);
            input::install(hwnd)?;
            observation::install(hwnd)?;
            trigger::install(hwnd)?;
            Ok(window)
        }
    }
    fn owns_hwnd(&self) -> bool {
        unsafe {
            IsWindow(Some(self.hwnd)).as_bool()
                && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA)
                    == (&*self.state as *const State) as isize
        }
    }
    pub fn is_alive(&self) -> bool {
        !self.state.closed.get() && self.owns_hwnd()
    }
    pub fn is_authorized(&self) -> bool {
        self.media.validate(&self.permit).is_ok()
    }
    pub fn change_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.is_alive().then(|| self.media.change_handle())
    }
    pub fn show(&self) -> Result<()> {
        self.refresh()?;
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Remote viewer window closed"));
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
    pub fn take_error(&self) -> Option<Error> {
        self.state.error.borrow_mut().take()
    }
    pub fn refresh(&self) -> Result<()> {
        if !self.is_alive() {
            return Ok(());
        }
        let result = self.state.with_host(|host| Ok(host.refresh()));
        unsafe {
            apply_update(self.hwnd, &self.state, result?)?;
            reschedule(self.hwnd, &self.state);
            input::refresh(self.hwnd, &self.state);
            observation::refresh(self.hwnd, &self.state);
            trigger::refresh(self.hwnd, &self.state);
        }
        Ok(())
    }
    pub fn close(&self) -> Result<()> {
        self.state.stop();
        if self.owns_hwnd() {
            unsafe {
                DestroyWindow(self.hwnd)?;
            }
        }
        Ok(())
    }
}
impl Drop for RelayWindow {
    fn drop(&mut self) {
        self.state.stop();
        if self.owns_hwnd() {
            unsafe {
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                let _ = DestroyWindow(self.hwnd);
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
                    style: CS_DBLCLKS,
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                    hCursor: LoadCursorW(None, IDC_ARROW)?,
                    lpszClassName: w!("Otter.Native.Relay"),
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
unsafe fn apply_update(hwnd: HWND, state: &State, update: Option<Update>) -> Result<()> {
    if state.closed.get()
        || GetWindowLongPtrW(hwnd, GWLP_USERDATA) != state as *const State as isize
    {
        return Ok(());
    }
    match update {
        Some(Update::Closed) => DestroyWindow(hwnd),
        Some(Update::Frame) if IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() => {
            if !InvalidateRect(Some(hwnd), None, false).as_bool() {
                return Err(Error::from_win32());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
unsafe fn reschedule(hwnd: HWND, state: &State) {
    if !state.closed.get() && state.host.borrow().is_some() && state.deferred.replace(false) {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if message == WM_NCCREATE {
        let create = &*(lp.0 as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
    }
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
    let Some(state) = pointer.as_ref() else {
        return DefWindowProcW(hwnd, message, wp, lp);
    };
    if let Some(result) = crate::desktop::session_end::dispatch_active(
        hwnd,
        &state.session_end_reason,
        message,
        wp,
        lp,
    ) {
        return result;
    }
    if input::handle(hwnd, state, message, wp, lp) {
        return LRESULT(0);
    }
    if observation::handle(hwnd, state, message, wp) {
        return LRESULT(0);
    }
    if trigger::handle(hwnd, state, message, wp) {
        return LRESULT(0);
    }
    let result = match message {
        WM_PAINT => paint(hwnd, state),
        WM_SIZE | WM_SHOWWINDOW => {
            let _ = InvalidateRect(Some(hwnd), None, false);
            Ok(())
        }
        WM_ERASEBKGND => return LRESULT(1),
        WM_DESTROY => {
            state.stop();
            Ok(())
        }
        WM_NCDESTROY => {
            state.stop();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            return DefWindowProcW(hwnd, message, wp, lp);
        }
        _ => return DefWindowProcW(hwnd, message, wp, lp),
    };
    if let Err(error) = result {
        *state.error.borrow_mut() = Some(error);
        state.stop();
        if GetWindowLongPtrW(hwnd, GWLP_USERDATA) == pointer as isize {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(hwnd);
        }
    }
    reschedule(hwnd, state);
    LRESULT(0)
}
unsafe fn paint(hwnd: HWND, state: &State) -> Result<()> {
    let mut paint = PAINTSTRUCT::default();
    let _ = BeginPaint(hwnd, &mut paint);
    let result = (|| {
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect)?;
        if IsIconic(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
            rect = RECT::default();
        }
        state.with_host(|host| {
            host.paint(D2D_SIZE_U {
                width: (rect.right - rect.left).max(0) as u32,
                height: (rect.bottom - rect.top).max(0) as u32,
            })?;
            Ok(if host.is_closed() {
                Update::Closed
            } else {
                Update::Idle
            })
        })
    })();
    let _ = EndPaint(hwnd, &paint);
    result.and_then(|update| apply_update(hwnd, state, update))
}

#[cfg(test)]
#[path = "composition_relay_window_tests.rs"]
mod tests;
