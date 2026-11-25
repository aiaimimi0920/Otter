//! Install WH_MOUSE_LL on demand only while visible Live windows with source control exist; viewing-only, hidden, and minimized periods have no global hook.
use super::*;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
struct Dpi(DPI_AWARENESS_CONTEXT);
impl Drop for Dpi {
    fn drop(&mut self) {
        unsafe {
            SetThreadDpiAwarenessContext(self.0);
        }
    }
}
struct Slot {
    hwnd: HWND,
    marker: isize,
    active: bool,
}
impl Slot {
    fn matches(&self, hwnd: HWND, marker: isize) -> bool {
        self.hwnd == hwnd && self.marker == marker
    }
}
thread_local! {
    static ROUTER: RefCell<Router> = RefCell::new(Router {
        hook: HHOOK::default(),
        windows: Vec::new(),
        last_error: None,
    });
    #[cfg(test)]
    static ROUTED: Cell<usize> = const { Cell::new(0) };
}
#[cfg(test)]
pub(super) fn routed() -> usize {
    ROUTED.get()
}
impl LiveWindow {
    #[cfg(test)]
    pub(in crate::desktop) fn wheel_routed_for_test(&self) -> usize {
        routed()
    }
}
#[cfg(test)]
pub(super) fn hook_installed_for_test() -> bool {
    ROUTER.with(|owner| !owner.borrow().hook.is_invalid())
}
struct Router {
    hook: HHOOK,
    windows: Vec<Slot>,
    last_error: Option<HRESULT>,
}
impl Router {
    unsafe fn refresh_hook(&mut self) -> Result<()> {
        let needed = !crate::desktop::session_end::blocks_input(WM_MOUSEWHEEL)
            && self.windows.iter().any(|slot| slot.active);
        if needed && self.hook.is_invalid() {
            #[cfg(test)]
            tests::hook_operation(true)?;
            self.hook = SetWindowsHookExW(WH_MOUSE_LL, Some(callback), None, 0)?;
        } else if !needed && !self.hook.is_invalid() {
            #[cfg(test)]
            tests::hook_operation(false)?;
            // Failed unregistration retains handle ownership; retry on the next synchronization without losing cleanup responsibility or installing duplicates.
            UnhookWindowsHookEx(self.hook)?;
            self.hook = HHOOK::default();
        }
        Ok(())
    }
    unsafe fn refresh(&mut self) {
        match self.refresh_hook() {
            Ok(()) => self.last_error = None,
            Err(error) => {
                if self.last_error.replace(error.code()) != Some(error.code()) {
                    eprintln!("native-live-wheel-hook: {error}");
                }
            }
        }
    }
}
impl Drop for Router {
    fn drop(&mut self) {
        if !self.hook.is_invalid() {
            unsafe {
                if let Err(error) = UnhookWindowsHookEx(self.hook) {
                    eprintln!("native-live-wheel-hook-cleanup: {error}");
                }
            }
        }
    }
}
pub(super) struct Registration(HWND, isize);
impl Registration {
    pub(super) unsafe fn new(hwnd: HWND) -> Result<Self> {
        ROUTER.with(|owner| {
            let mut owner = owner.borrow_mut();
            owner
                .windows
                .try_reserve(1)
                .map_err(|_| Error::new(E_OUTOFMEMORY, "Live wheel registrations exhausted"))?;
            let marker = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            owner.windows.push(Slot {
                hwnd,
                marker,
                active: false,
            });
            Ok(Self(hwnd, marker))
        })
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        ROUTER.with(|owner| {
            let mut owner = owner.borrow_mut();
            owner.windows.retain(|slot| !slot.matches(self.0, self.1));
            unsafe {
                owner.refresh();
                if owner.windows.is_empty() && !owner.hook.is_invalid() {
                    // The final window has no later synchronization opportunity; perform just one additional bounded cleanup retry.
                    owner.refresh();
                }
            }
        });
    }
}
pub(super) unsafe fn sync(hwnd: HWND, state: &State) {
    let active = !state.closed.get()
        && !state.group_locked.get()
        && !state.group_hidden.get()
        && !state.session_end_reason.get()
        && !crate::desktop::session_end::blocks_input(WM_MOUSEWHEEL)
        && IsWindowVisible(hwnd).as_bool()
        && !IsIconic(hwnd).as_bool()
        && state
            .input
            .try_borrow()
            .is_ok_and(|input| input.wheel_wanted());
    ROUTER.with(|owner| {
        if let Ok(mut owner) = owner.try_borrow_mut() {
            let marker = state as *const State as isize;
            if let Some(slot) = owner
                .windows
                .iter_mut()
                .find(|slot| slot.matches(hwnd, marker))
            {
                slot.active = active;
                // Unchanged active may still follow failed installation/unregistration; attempt at most once per synchronization event.
                owner.refresh();
            }
        }
    });
}
unsafe extern "system" fn callback(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // Nonwheel messages, including high-frequency WM_MOUSEMOVE, return directly without catch_unwind, DPI changes, or window queries.
    if code != HC_ACTION as i32
        || (wp.0 != WM_MOUSEWHEEL as usize && wp.0 != WM_MOUSEHWHEEL as usize)
    {
        return CallNextHookEx(None, code, wp, lp);
    }
    let handled = std::panic::catch_unwind(|| {
        let mouse = &*(lp.0 as *const MSLLHOOKSTRUCT);
        // Windows invokes hooks in the callback's registered default context; explicitly keep hit/delivery coordinates in physical pixels.
        let _dpi = Dpi(SetThreadDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ));
        // LL hooks provide physical screen coordinates; callback DPI context may differ from the UI context that created Live.
        let hwnd = WindowFromPhysicalPoint(mouse.pt);
        let marker = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        if marker == 0 {
            return false;
        }
        let routed = ROUTER.with(|owner| {
            owner
                .borrow()
                .windows
                .iter()
                .any(|slot| slot.matches(hwnd, marker) && slot.active)
        });
        if !routed {
            return false;
        }
        let state = &*(marker as *const State);
        #[cfg(test)]
        eprintln!("native-live-wheel-route: hwnd={hwnd:?}");
        if state.closed.get()
            || state.menu_open.get()
            || state.group_locked.get()
            || state.group_hidden.get()
            || state.session_end_reason.get()
            || crate::desktop::session_end::blocks_input(WM_MOUSEWHEEL)
        {
            return false;
        }
        let mut flags = 0;
        for (key, mask) in [
            (VK_LBUTTON, 1),
            (VK_RBUTTON, 2),
            (VK_SHIFT, 4),
            (VK_CONTROL, 8),
            (VK_MBUTTON, 16),
        ] {
            if GetAsyncKeyState(key.0 as i32) < 0 {
                flags |= mask;
            }
        }
        let point = (mouse.pt.x as u16 as u32) | ((mouse.pt.y as u16 as u32) << 16);
        PostMessageW(
            Some(hwnd),
            wp.0 as u32,
            WPARAM((mouse.mouseData & 0xffff0000) as usize | flags),
            LPARAM(point as isize),
        )
        .is_ok()
    })
    .unwrap_or(false);
    if handled {
        #[cfg(test)]
        ROUTED.set(ROUTED.get() + 1);
        LRESULT(1)
    } else {
        CallNextHookEx(None, code, wp, lp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{desktop::Apartment, live_frames::Source};
    #[test]
    fn retired_registration_cannot_remove_reused_hwnd_owner() {
        let hwnd = HWND(1usize as *mut _);
        ROUTER.with(|owner| {
            let mut owner = owner.borrow_mut();
            assert!(owner.windows.is_empty());
            owner.windows.push(Slot {
                hwnd,
                marker: 11,
                active: false,
            });
            owner.windows.push(Slot {
                hwnd,
                marker: 22,
                active: false,
            });
            assert!(owner.windows[1].matches(hwnd, 22));
            assert!(!owner.windows[0].matches(hwnd, 22));
        });
        drop(Registration(hwnd, 11));
        ROUTER.with(|owner| {
            let owner = owner.borrow();
            assert_eq!(owner.windows.len(), 1);
            assert!(owner.windows[0].matches(hwnd, 22));
        });
        drop(Registration(hwnd, 22));
        assert!(ROUTER.with(|owner| owner.borrow().windows.is_empty()));
    }
    thread_local! {
        static FAIL_NEXT: Cell<Option<bool>> = const { Cell::new(None) };
        static HOOK_CALLS: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
    }
    pub(super) fn hook_operation(install: bool) -> Result<()> {
        let (installs, uninstalls) = HOOK_CALLS.get();
        HOOK_CALLS.set((
            installs + usize::from(install),
            uninstalls + usize::from(!install),
        ));
        if FAIL_NEXT.get() == Some(install) {
            FAIL_NEXT.set(None);
            return Err(Error::new(E_FAIL, "injected Live wheel hook failure"));
        }
        Ok(())
    }

    #[test]
    fn hook_failures_retry_without_activity_edges_and_preserve_unhook_ownership() {
        let _apartment = Apartment::new().unwrap();
        unsafe {
            let source = Source::new();
            let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
            window.show().unwrap();
            window.allow_input().unwrap();
            let before = HOOK_CALLS.get();
            FAIL_NEXT.set(Some(true));
            window.request_default_input();
            assert_eq!(HOOK_CALLS.get(), (before.0 + 1, before.1));
            assert!(!hook_installed_for_test());
            assert_eq!(ROUTER.with(|owner| owner.borrow().last_error), Some(E_FAIL));
            sync(window.hwnd, &window.state);
            assert_eq!(HOOK_CALLS.get(), (before.0 + 2, before.1));
            assert!(hook_installed_for_test());
            assert_eq!(ROUTER.with(|owner| owner.borrow().last_error), None);

            window.state.group_locked.set(true);
            FAIL_NEXT.set(Some(false));
            sync(window.hwnd, &window.state);
            assert_eq!(HOOK_CALLS.get(), (before.0 + 2, before.1 + 1));
            assert!(hook_installed_for_test(), "failed unhook keeps its owner");
            sync(window.hwnd, &window.state);
            assert_eq!(HOOK_CALLS.get(), (before.0 + 2, before.1 + 2));
            assert!(!hook_installed_for_test());
            window.state.group_locked.set(false);
            sync(window.hwnd, &window.state);
            assert!(hook_installed_for_test());
            let before_drop = HOOK_CALLS.get();
            FAIL_NEXT.set(Some(false));
            drop(window);
            assert!(!hook_installed_for_test());
            assert_eq!(HOOK_CALLS.get(), (before_drop.0, before_drop.1 + 2));
        }
    }

    #[test]
    fn session_end_query_and_cancel_refresh_hook_before_early_return() {
        use crate::desktop::session_end::{with_owner, with_query, Control};
        let _apartment = Apartment::new().unwrap();
        let control = std::rc::Rc::new(Control::default());
        with_owner(&control, || unsafe {
            let source = Source::new();
            let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
            window.show().unwrap();
            window.allow_input().unwrap();
            window.request_default_input();
            assert!(hook_installed_for_test());
            assert_eq!(
                with_query(
                    &control,
                    || true,
                    || SendMessageW(window.hwnd, WM_QUERYENDSESSION, None, None).0,
                ),
                1
            );
            assert!(control.paused());
            assert!(!window.state.session_end_reason.get());
            assert!(
                !hook_installed_for_test(),
                "accepted shutdown pauses routing"
            );
            SendMessageW(window.hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
            assert!(!control.paused());
            assert!(hook_installed_for_test());
            assert_eq!(
                SendMessageW(window.hwnd, WM_QUERYENDSESSION, None, None).0,
                0
            );
            assert!(window.state.session_end_reason.get());
            assert!(
                !hook_installed_for_test(),
                "rejected shutdown reason pauses routing"
            );
            SendMessageW(window.hwnd, WM_ENDSESSION, Some(WPARAM(0)), None);
            assert!(hook_installed_for_test());
            drop(window);
            assert!(!hook_installed_for_test());
        });
    }

    #[test]
    fn rendering_error_unhooks_before_window_userdata_is_cleared() {
        let _apartment = Apartment::new().unwrap();
        unsafe {
            let source = Source::new();
            let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
            window.show().unwrap();
            window.allow_input().unwrap();
            window.request_default_input();
            super::super::tests::publish(&source);
            SendMessageW(window.hwnd, WM_PAINT, None, None);
            assert!(window.is_alive());
            assert!(hook_installed_for_test());
            super::super::tests::publish(&source);
            SendMessageW(window.hwnd, WM_PAINT, None, None);
            assert!(!window.is_alive());
            assert!(window.take_error().is_some());
            assert!(
                !hook_installed_for_test(),
                "closed owner may outlive its HWND"
            );
        }
    }

    #[test]
    fn view_only_and_hidden_live_windows_never_install_global_mouse_hook() {
        let _apartment = Apartment::new().unwrap();
        unsafe {
            let source = Source::new();
            let window = LiveWindow::new(source.subscribe().unwrap(), 128, 128).unwrap();
            assert!(!hook_installed_for_test());
            window.show().unwrap();
            assert!(
                !hook_installed_for_test(),
                "view-only Live pin must not install WH_MOUSE_LL"
            );
            window.allow_input().unwrap();
            assert!(!hook_installed_for_test());
            window.request_default_input();
            assert!(
                hook_installed_for_test(),
                "visible interactive Live pin arms WH_MOUSE_LL"
            );
            let _ = ShowWindow(window.hwnd, SW_HIDE);
            sync(window.hwnd, &window.state);
            assert!(!hook_installed_for_test(), "hiding window unhooks");
            window.show().unwrap();
            assert!(hook_installed_for_test());
            input::cancel(window.hwnd, &window.state);
            assert!(!hook_installed_for_test(), "disabling input unhooks");
            window.request_default_input();
            assert!(hook_installed_for_test());
            drop(window);
            assert!(!hook_installed_for_test(), "drop unhooks cleanly");
        }
    }
}
