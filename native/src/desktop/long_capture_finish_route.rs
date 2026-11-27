//! Sole UI-thread scrolling-capture finish route; shared capture identity, window properties, and increasing generations jointly reject late messages.
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use windows::{
    core::{w, Error, Result},
    Win32::{Foundation::*, System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*},
};

pub(super) const MESSAGE: u32 = WM_APP + 0x5c1;
const OWNER: windows::core::PCWSTR = w!("Otter.LongCapture.Finish.Owner");
thread_local! {
    static ACTIVE: RefCell<Weak<Route>> = const { RefCell::new(Weak::new()) };
    static SEQUENCE: Cell<usize> = const { Cell::new(0) };
}
struct Route {
    hwnd: HWND,
    token: usize,
    capture: Rc<Cell<bool>>,
    queued: Cell<bool>,
}
impl Route {
    unsafe fn alive(&self) -> bool {
        IsWindow(Some(self.hwnd)).as_bool()
            && GetWindowThreadProcessId(self.hwnd, None) == GetCurrentThreadId()
            && GetPropW(self.hwnd, OWNER).0 as usize == self.token
    }
}
impl Drop for Route {
    fn drop(&mut self) {
        unsafe {
            if self.alive() {
                let _ = RemovePropW(self.hwnd, OWNER);
            }
            crate::desktop::shortcut_registry::finish_changed();
        }
    }
}
pub(super) struct Registration(Rc<Route>);
impl Registration {
    pub unsafe fn new(hwnd: HWND, capture: Rc<Cell<bool>>) -> Result<Self> {
        if !IsWindow(Some(hwnd)).as_bool()
            || GetWindowThreadProcessId(hwnd, None) != GetCurrentThreadId()
            || ACTIVE.with(|active| active.borrow().upgrade().is_some_and(|route| route.alive()))
        {
            return Err(Error::new(E_FAIL, "Long capture finish route unavailable"));
        }
        let token = SEQUENCE.with(|sequence| {
            let token = sequence
                .get()
                .checked_add(1)
                .ok_or_else(|| Error::new(E_FAIL, "Long capture finish identity exhausted"))?;
            sequence.set(token);
            Ok::<_, Error>(token)
        })?;
        SetPropW(hwnd, OWNER, Some(HANDLE(token as *mut _)))?;
        let route = Rc::new(Route {
            hwnd,
            token,
            capture,
            queued: Cell::new(false),
        });
        ACTIVE.with(|active| *active.borrow_mut() = Rc::downgrade(&route));
        crate::desktop::shortcut_registry::finish_changed();
        Ok(Self(route))
    }
    pub unsafe fn accept(&self, hwnd: HWND, wp: WPARAM, lp: LPARAM) -> bool {
        if self.0.hwnd != hwnd
            || wp.0 != self.0.token
            || lp.0 != 0
            || !self.0.capture.get()
            || !self.0.alive()
        {
            return false;
        }
        self.0.queued.set(false);
        true
    }
}
pub(in crate::desktop) unsafe fn active(capture: &Rc<Cell<bool>>) -> bool {
    capture.get()
        && ACTIVE.with(|active| {
            active
                .borrow()
                .upgrade()
                .is_some_and(|route| Rc::ptr_eq(capture, &route.capture) && route.alive())
        })
}
/// Do not reenter dialogs synchronously; enqueue at most one notice, collectible only by in-flight capture in the same Application.
pub(in crate::desktop) unsafe fn request(capture: &Rc<Cell<bool>>) -> bool {
    let route = ACTIVE.with(|active| active.borrow().upgrade());
    let Some(route) = route else {
        return false;
    };
    if !capture.get() || !Rc::ptr_eq(capture, &route.capture) || !route.alive() {
        return false;
    }
    if !route.queued.replace(true) {
        if let Err(error) = PostMessageW(Some(route.hwnd), MESSAGE, WPARAM(route.token), LPARAM(0))
        {
            route.queued.set(false);
            eprintln!("native-long-capture-finish: {error}");
        }
    }
    // Even system-queue failure must not downgrade a second keystroke into another selection; users can still click Finish or retry.
    true
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "long_capture_finish_route_tests.rs"]
mod tests;
