//! Dedicated Live HWND; no editable-pin State reuse, capture startup, or application-message-loop ownership.
use super::{
    host::{Host, Presented, Update},
    notification, Texture,
};
use crate::live_frames::Consumer;
#[path = "composition_live_art.rs"]
mod art;
#[path = "composition_live_freeze_geometry.rs"]
mod freeze_geometry;
#[path = "composition_live_groups.rs"]
pub(in crate::desktop) mod groups;
#[path = "composition_live_window_input.rs"]
mod input;
#[path = "composition_live_local.rs"]
pub(in crate::desktop) mod local;
#[path = "composition_live_menu.rs"]
mod menu;
#[path = "composition_live_pin.rs"]
mod pin;
#[path = "composition_live_placement.rs"]
mod placement;
#[path = "composition_live_popup_window.rs"]
mod popup_window;
#[path = "composition_live_publication.rs"]
pub(in crate::desktop) mod publication;
#[path = "composition_live_wheel.rs"]
mod wheel;
use std::{
    cell::{Cell, RefCell},
    sync::OnceLock,
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

struct State {
    popup: Cell<bool>,
    popup_noactivate: Cell<bool>,
    popup_peers: RefCell<Vec<HWND>>,
    popup_focus: RefCell<input::PopupFocus>,
    pin: pin::Pin,
    chrome: crate::desktop::application::chrome::Binding,
    session_end_reason: Cell<bool>,
    publication: publication::Control,
    groups: RefCell<groups::Control>,
    group_locked: Cell<bool>,
    group_hidden: Cell<bool>,
    group_was_visible: Cell<bool>,
    recycle_requested: Cell<bool>,
    recycle_on_close: Cell<bool>,
    freeze_requested: Cell<bool>,
    snapshot_requested: Cell<bool>,
    link_requested: Cell<bool>,
    art_requested: Cell<bool>,
    opacity: Cell<f32>,
    drag: Cell<Option<local::drag::Drag>>,
    drag_starting: Cell<bool>,
    drag_revision: Cell<u64>,
    menu_open: Cell<bool>,
    unlink_requested: Cell<bool>,
    host: RefCell<Option<Host>>,
    presented: Presented,
    error: RefCell<Option<Error>>,
    closed: Cell<bool>,
    deferred: Cell<bool>,
    input: RefCell<input::Control>,
}
impl State {
    // Win32/GPU calls may reenter; hold no RefCell borrows across calls and do not reattach owners during closure.
    fn with_host<T>(&self, operation: impl FnOnce(&mut Host) -> Result<T>) -> Result<Option<T>> {
        let Some(mut host) = self.host.borrow_mut().take() else {
            self.deferred.set(true);
            return Ok(None);
        };
        let result = operation(&mut host);
        if !self.closed.get() {
            *self.host.borrow_mut() = Some(host);
        }
        result.map(Some)
    }
    fn stop(&self) {
        self.presented.clear();
        self.art_requested.set(false);
        self.drag.set(None);
        self.drag_starting.set(false);
        self.input.borrow_mut().cancel();
        self.closed.set(true);
        let host = self.host.borrow_mut().take();
        drop(host);
    }
}

pub struct LiveWindow {
    wheel: Option<wheel::Registration>,
    unit_id: crate::workspace_id::UnitId,
    hwnd: HWND,
    state: Box<State>,
}
impl LiveWindow {
    pub(in crate::desktop) fn bind_workspace_chrome(
        &self,
        binding: std::rc::Weak<crate::desktop::application::chrome::Ui>,
    ) {
        *self.state.chrome.borrow_mut() = binding;
    }
    pub(in crate::desktop) fn enable_recycle_close(&self) {
        self.state.recycle_on_close.set(true);
    }
    pub fn take_recycle_request(&self) -> bool {
        self.state.recycle_requested.replace(false)
    }
    /// The application has prepared a freeze record; on failure, do not publish reclamation results while HWND still belongs to this owner.
    pub(in crate::desktop) fn close_for_recycle(&self) -> Result<()> {
        if !self.is_alive() {
            return Err(Error::new(E_ABORT, "Live recycle window already closed"));
        }
        let result = unsafe { DestroyWindow(self.hwnd) };
        if self.owns_hwnd() {
            return Err(result
                .err()
                .unwrap_or_else(|| Error::new(E_FAIL, "Live recycle window remained open")));
        }
        Ok(())
    }
    pub fn unit_id(&self) -> &crate::workspace_id::UnitId {
        &self.unit_id
    }
    pub fn take_freeze_request(&self) -> bool {
        self.state.freeze_requested.replace(false)
    }
    pub fn take_snapshot_request(&self) -> bool {
        self.state.snapshot_requested.replace(false)
    }
    pub fn take_link_request(&self) -> bool {
        self.state.link_requested.replace(false)
    }
    pub fn opacity(&self) -> f32 {
        self.state.opacity.get()
    }
    pub fn take_unlink_request(&self) -> bool {
        self.state.unlink_requested.replace(false)
    }
    pub(super) fn native_handle(&self) -> HWND {
        self.hwnd
    }
    #[cfg(test)]
    pub(in crate::desktop) fn hwnd_for_test(&self) -> HWND {
        self.hwnd
    }
    #[cfg(test)]
    pub(in crate::desktop) fn frame_for_test(
        &self,
    ) -> Option<std::sync::Arc<crate::live_frames::Frame<Texture>>> {
        self.state.host.borrow().as_ref()?.frame_for_test()
    }
    /// Create a hidden window and set system capture exclusion before binding sources; callers own DPI context and explicit user authorization.
    pub fn new(consumer: Consumer<Texture>, width: u32, height: u32) -> Result<Self> {
        super::super::validate_size(D2D_SIZE_U { width, height })?;
        unsafe {
            register()?;
            let unit_id = crate::desktop::document_state::new_unit_id()?;
            let state = Box::new(State {
                popup: Cell::new(false),
                popup_noactivate: Cell::new(false),
                popup_peers: RefCell::new(Vec::new()),
                popup_focus: Default::default(),
                pin: Default::default(),
                chrome: Default::default(),
                session_end_reason: Cell::new(false),
                publication: publication::Control::default(),
                groups: RefCell::default(),
                group_locked: Cell::new(false),
                group_hidden: Cell::new(false),
                group_was_visible: Cell::new(false),
                recycle_requested: Cell::new(false),
                recycle_on_close: Cell::new(false),
                freeze_requested: Cell::new(false),
                snapshot_requested: Cell::new(false),
                link_requested: Cell::new(false),
                art_requested: Cell::new(false),
                opacity: Cell::new(1.0),
                drag: Cell::new(None),
                drag_starting: Cell::new(false),
                drag_revision: Cell::new(0),
                menu_open: Cell::new(false),
                unlink_requested: Cell::new(false),
                host: RefCell::new(None),
                presented: Presented::default(),
                error: RefCell::new(None),
                closed: Cell::new(false),
                deferred: Cell::new(false),
                input: RefCell::new(input::Control::default()),
            });
            let hwnd = CreateWindowExW(
                WS_EX_NOREDIRECTIONBITMAP | WS_EX_TOOLWINDOW,
                w!("Otter.Native.Live"),
                w!("Otter Live"),
                WS_POPUP | WS_SYSMENU,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width as i32,
                height as i32,
                None,
                None,
                Some(HINSTANCE(GetModuleHandleW(None)?.0)),
                Some((&*state as *const State).cast()),
            )?;
            let mut window = Self {
                wheel: None,
                hwnd,
                state,
                unit_id,
            };
            input::install(hwnd)?;
            window.state.input.borrow_mut().hover =
                Some(crate::live_hover_owner::Owner::new(hwnd)?);
            window.wheel = Some(wheel::Registration::new(hwnd)?);
            art::install(hwnd)?;
            crate::desktop::application::chrome::ports::install_live(hwnd)?;
            publication::install(hwnd)?;
            window.state.groups.borrow_mut().menu = Some(groups::install(hwnd)?);
            menu::install(hwnd)?;
            SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE)?;
            *window.state.host.borrow_mut() = Some(Host::bind_presented(
                hwnd,
                consumer,
                window.state.presented.clone(),
            )?);
            Ok(window)
        }
    }
    pub fn is_alive(&self) -> bool {
        !self.state.closed.get() && self.owns_hwnd()
    }
    fn owns_hwnd(&self) -> bool {
        unsafe {
            IsWindow(Some(self.hwnd)).as_bool()
                && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA)
                    == (&*self.state as *const State) as isize
        }
    }
    pub fn show(&self) -> Result<()> {
        if !self.is_alive() {
            return Err(Error::new(E_FAIL, "Live window is closed"));
        }
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            wheel::sync(self.hwnd, &self.state);
        }
        Ok(())
    }
    pub fn take_error(&self) -> Option<Error> {
        self.state.error.borrow_mut().take()
    }
    /// Transfer only the budget owner; no GPU readback in window callbacks. Return None during reentrant occupancy, never stale caches.
    pub fn snapshot_frame(
        &self,
    ) -> Result<Option<std::sync::Arc<crate::live_frames::Frame<Texture>>>> {
        if !self.is_alive() {
            return Ok(None);
        }
        self.state
            .with_host(Host::snapshot_frame)
            .map(Option::flatten)
    }
    /// Compensate for missed notifications through existing application events; without later events, guaranteed closure is not promised.
    pub fn refresh(&self) -> Result<()> {
        if !self.is_alive() {
            return Ok(());
        }
        unsafe {
            wheel::sync(self.hwnd, &self.state);
            handle_update(self.hwnd, self.state.with_host(Host::refresh)?)?;
            reschedule(self.hwnd, &self.state);
            Ok(())
        }
    }
}
impl Drop for LiveWindow {
    fn drop(&mut self) {
        self.wheel.take();
        let alive = self.owns_hwnd();
        self.state.stop();
        if alive {
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
            let result = (|| -> Result<()> {
                let class = WNDCLASSW {
                    style: CS_DBLCLKS,
                    lpfnWndProc: Some(procedure),
                    hInstance: HINSTANCE(GetModuleHandleW(None)?.0),
                    hCursor: LoadCursorW(None, IDC_ARROW)?,
                    lpszClassName: w!("Otter.Native.Live"),
                    ..Default::default()
                };
                if RegisterClassW(&class) == 0 {
                    return Err(Error::from_win32());
                }
                Ok(())
            })();
            result.map_err(|error| error.code())
        })
        .map_err(Error::from_hresult)
}
unsafe fn handle_update(hwnd: HWND, update: Option<Update>) -> Result<()> {
    match update {
        Some(Update::Closed) => DestroyWindow(hwnd),
        Some(Update::Frame) => {
            if !InvalidateRect(Some(hwnd), None, false).as_bool() {
                return Err(Error::from_win32());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
unsafe fn reschedule(hwnd: HWND, state: &State) -> bool {
    // The system queue consumed the reentrant notification while outer drawing borrowed Host; add one invalidation region after outer EndPaint.
    if !state.closed.get() && state.host.borrow().is_some() && state.deferred.replace(false) {
        return InvalidateRect(Some(hwnd), None, false).as_bool();
    }
    false
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
    if state.popup_noactivate.get() && message == WM_MOUSEACTIVATE {
        return LRESULT(MA_NOACTIVATE as isize);
    }
    crate::desktop::application::chrome::geometry(&state.chrome, hwnd, message);
    if let Some(result) = crate::desktop::session_end::dispatch_active(
        hwnd,
        &state.session_end_reason,
        message,
        wp,
        lp,
    ) {
        if matches!(
            message,
            WM_QUERYENDSESSION | WM_ENDSESSION | WM_WINDOWPOSCHANGED | WM_SIZE
        ) {
            wheel::sync(hwnd, state);
        }
        return result;
    }
    // Static sources post no further frames on becoming visible; the default procedure still dispatches WM_SIZE/WM_MOVE.
    if message == WM_WINDOWPOSCHANGED && lp.0 != 0 {
        if (*(lp.0 as *const WINDOWPOS)).flags.contains(SWP_SHOWWINDOW) {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        wheel::sync(hwnd, state);
    }
    let result = if let Some(result) = input::handle(hwnd, state, message, wp, lp) {
        result
    } else {
        match message {
            WM_SYSCOMMAND if publication::handles(wp.0 & 0xfff0) => {
                publication::queue(state, wp.0 & 0xfff0);
                Ok(())
            }
            WM_SYSCOMMAND if groups::handles(wp.0 & 0xfff0) => {
                groups::queue(state, wp.0 & 0xfff0);
                Ok(())
            }
            WM_SYSCOMMAND
                if state.group_locked.get()
                    && matches!((wp.0 & 0xfff0) as u32, SC_MOVE | SC_SIZE) =>
            {
                Ok(())
            }
            // Explicit discard preserves an exit path: missing first frames or old-workspace provenance must not trap the window.
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1f90 => {
                if !state.group_locked.get() && !state.group_hidden.get() {
                    state.unlink_requested.set(true);
                }
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fa0 => {
                if !state.group_locked.get() && !state.group_hidden.get() {
                    state.link_requested.set(true);
                }
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fb0 => {
                if !state.group_locked.get() && !state.group_hidden.get() {
                    state.snapshot_requested.set(true);
                }
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fc0 => DestroyWindow(hwnd),
            WM_CLOSE if state.recycle_on_close.get() => {
                state.recycle_requested.set(true);
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fd0 => {
                state.recycle_requested.set(true);
                Ok(())
            }
            WM_SYSCOMMAND if wp.0 & 0xfff0 == 0x1fe0 => {
                state.freeze_requested.set(true);
                Ok(())
            }
            notification::MESSAGE => state
                .with_host(|host| host.receive(message, wp, lp))
                .and_then(|update| handle_update(hwnd, update)),
            WM_PAINT => paint(hwnd, state),
            WM_SIZE => {
                if wp.0 == SIZE_MINIMIZED as usize {
                    let _ = state.with_host(|host| {
                        host.suspend();
                        Ok(())
                    });
                }
                wheel::sync(hwnd, state);
                let _ = InvalidateRect(Some(hwnd), None, false);
                Ok(())
            }
            WM_ERASEBKGND => return LRESULT(1),
            WM_DESTROY => {
                state.stop();
                wheel::sync(hwnd, state);
                Ok(())
            }
            WM_NCDESTROY => {
                state.stop();
                wheel::sync(hwnd, state);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                return DefWindowProcW(hwnd, message, wp, lp);
            }
            _ => return DefWindowProcW(hwnd, message, wp, lp),
        }
    };
    if let Err(error) = result {
        *state.error.borrow_mut() = Some(error);
        state.stop();
        wheel::sync(hwnd, state);
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
        let mut client = RECT::default();
        GetClientRect(hwnd, &mut client)?;
        // Minimized windows may still return nonzero client rectangles; this must not trigger swap-chain recreation.
        if IsIconic(hwnd).as_bool() {
            client = RECT::default();
        }
        state.with_host(|host| {
            host.set_opacity(state.opacity.get())?;
            host.select_pin(state.pin.selected.get());
            host.set_chrome_visible(crate::desktop::application::chrome::live_frame(
                &state.chrome,
            ));
            let size = D2D_SIZE_U {
                width: (client.right - client.left).max(0) as u32,
                height: (client.bottom - client.top).max(0) as u32,
            };
            host.paint(size)?;
            Ok(if host.is_closed() {
                Update::Closed
            } else {
                Update::Idle
            })
        })
    })();
    let _ = EndPaint(hwnd, &paint);
    result.and_then(|update| handle_update(hwnd, update))
}

#[cfg(test)]
#[path = "composition_live_window_tests.rs"]
mod tests;
