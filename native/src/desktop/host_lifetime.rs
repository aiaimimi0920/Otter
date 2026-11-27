//! UI-thread COM/HWND lifetimes and host counts; tool panels are not independent documents and cannot decide application exit.
use std::cell::Cell;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

pub(super) struct Apartment;
impl Apartment {
    pub(super) fn new() -> windows::core::Result<Self> {
        unsafe {
            use windows::Win32::System::Com::*;
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Com::CoUninitialize();
        }
    }
}

// Owned by the creating thread; Drop after explicit destruction must not destroy HWND again.
pub(super) struct Window(pub(super) HWND);
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(Some(self.0)).as_bool() {
                let _ = super::smoke_timing::run("window-destroy", || DestroyWindow(self.0));
            }
        }
    }
}

/// Fix HWND identity before routing into State; duplicate candidates install no pointer, and failure cleanup must not touch original documents.
pub(super) unsafe fn attach(hwnd: HWND, lp: LPARAM) -> bool {
    let create = &*(lp.0 as *const CREATESTRUCTW);
    let pointer = create.lpCreateParams as *const super::State;
    if pointer.is_null() {
        return true; // Unbound hidden candidates use only DefWindowProc and own no document registration.
    }
    let state = &*pointer;
    if !state.lifetime.bind(hwnd) {
        return false;
    }
    SetLastError(WIN32_ERROR(0));
    if SetWindowLongPtrW(hwnd, GWLP_USERDATA, pointer as isize) == 0
        && GetLastError() != WIN32_ERROR(0)
    {
        state.lifetime.release(hwnd);
        return false;
    }
    true
}

pub(super) unsafe fn owns(hwnd: HWND) -> bool {
    let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const super::State;
    !pointer.is_null() && (*pointer).lifetime.owns(hwnd)
}

pub(super) unsafe fn dispatch(
    hwnd: HWND,
    state: &super::State,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> Option<LRESULT> {
    match msg {
        WM_CREATE => {
            if !state.lifetime.created() {
                return Some(LRESULT(-1));
            }
            windows::Win32::UI::Shell::DragAcceptFiles(hwnd, true);
            Some(LRESULT(0))
        }
        WM_DESTROY => {
            state.priority_watcher.borrow_mut().take();
            if state.lifetime.destroyed() {
                PostQuitMessage(0);
            }
            Some(LRESULT(0))
        }
        WM_NCDESTROY => {
            state.click.retire();
            state.canvas.borrow_mut().close_render();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            state.lifetime.release(hwnd);
            Some(DefWindowProcW(hwnd, msg, wp, lp))
        }
        _ => None,
    }
}

thread_local! {
    static LIVE: Cell<usize> = const { Cell::new(0) };
}

#[derive(Default)]
pub(super) struct Registration {
    registered: Cell<bool>,
    bound: Cell<HWND>,
    publication: std::cell::RefCell<Option<std::rc::Rc<()>>>,
    // Register/destroy on the HWND-owning UI thread; forbid cross-thread state transfer.
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}

impl Registration {
    fn bind(&self, hwnd: HWND) -> bool {
        if hwnd.is_invalid() || self.registered.get() || !self.bound.get().is_invalid() {
            return false;
        }
        self.bound.set(hwnd);
        self.publication.replace(Some(std::rc::Rc::new(())));
        true
    }

    pub(super) fn publication(&self) -> std::rc::Weak<()> {
        self.publication
            .borrow()
            .as_ref()
            .map(std::rc::Rc::downgrade)
            .unwrap_or_default()
    }

    pub fn owns(&self, hwnd: HWND) -> bool {
        !hwnd.is_invalid() && self.bound.get() == hwnd
    }

    pub fn is_sole_host(&self) -> bool {
        self.registered.get() && LIVE.with(|live| live.get() == 1)
    }

    fn release(&self, hwnd: HWND) {
        if self.owns(hwnd) {
            self.bound.set(HWND::default());
            self.publication.take();
        }
    }

    pub fn created(&self) -> bool {
        if self.registered.get() {
            return false;
        }
        LIVE.with(|live| {
            let Some(next) = live.get().checked_add(1) else {
                return false;
            };
            live.set(next);
            self.registered.set(true);
            true
        })
    }

    /// Report the last window finished only after an actually registered host closes.
    pub fn destroyed(&self) -> bool {
        self.publication.take();
        if !self.registered.replace(false) {
            return false;
        }
        LIVE.with(|live| {
            let next = live.get().saturating_sub(1);
            live.set(next);
            next == 0
        })
    }
}

#[cfg(test)]
#[path = "host_binding_tests.rs"]
mod binding_tests;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_last_registered_host_requests_exit_and_duplicates_do_not_change_count() {
        let first = Registration::default();
        let second = Registration::default();
        let failed = Registration::default();
        assert!(!failed.destroyed());
        assert!(first.created());
        assert!(!first.created());
        assert!(second.created());
        assert!(!first.destroyed());
        assert!(!first.destroyed());
        assert!(second.destroyed());
        assert!(!second.destroyed());
        assert_eq!(LIVE.with(Cell::get), 0);
    }
}
