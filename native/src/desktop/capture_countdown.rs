//! One countdown on the workspace UI thread. A timer never owns input or captures pixels.
use super::{capture_input, tool_panel_availability::Availability, State, Window};
use std::{
    cell::{Cell, RefCell},
    time::{Duration, Instant},
};
use windows::{
    core::{w, Error, Result, HSTRING},
    Win32::{
        Foundation::*,
        Graphics::{Dwm::DwmFlush, Gdi::*},
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

pub(super) const THREE: usize = 0x1ef1;
pub(super) const FIVE: usize = 0x1ef2;
pub(super) const CANCEL: usize = 0x1ef3;
struct Pending {
    host: HWND,
    timer: usize,
    deadline: Instant,
    hud: Window,
    shown: u32,
}
impl Drop for Pending {
    fn drop(&mut self) {
        unsafe {
            let _ = KillTimer(Some(self.host), self.timer);
        }
    }
}
thread_local! {
    static ACTIVE:RefCell<Option<Pending>>=const{RefCell::new(None)};
    // Never reuse timer IDs: KillTimer does not remove already queued WM_TIMER messages.
    static NEXT:Cell<usize>=const{Cell::new(0x33000000)};
}
pub(super) fn active() -> bool {
    ACTIVE.with_borrow(|p| p.is_some())
}
pub(super) fn cancel() {
    drop(ACTIVE.with_borrow_mut(Option::take));
}
pub(super) fn ready(state: &State) -> bool {
    !state.exit_requested.get()
        && !state.session_end.paused()
        && Availability::capture(state).is_some_and(|a| a.allows(capture_input::NEW_PIN))
}
fn label(seconds: u32) -> HSTRING {
    HSTRING::from(crate::ui_text::format(
        "capture-countdown-badge",
        &[("seconds", seconds.to_string().into())],
    ))
}
unsafe fn badge(seconds: u32) -> Result<Window> {
    let mut cursor = POINT::default();
    let _ = GetCursorPos(&mut cursor);
    let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as _,
        ..Default::default()
    };
    GetMonitorInfoW(monitor, &mut info).ok()?;
    // The badge is not owned by a document HWND: parent destruction cannot silently
    // invalidate this RAII handle. WM_DESTROY on its timer owner cancels it explicitly.
    let hud = Window(CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
        w!("STATIC"),
        &label(seconds),
        WS_DISABLED | WS_POPUP | WINDOW_STYLE(0x201),
        info.rcWork.left,
        info.rcWork.top,
        1,
        1,
        None,
        None,
        None,
        None,
    )?);
    SetPropW(hud.0, w!("NonRudeHWND"), Some(HANDLE(1usize as *mut _)))?;
    SetWindowDisplayAffinity(hud.0, WDA_EXCLUDEFROMCAPTURE)?;
    SetLayeredWindowAttributes(hud.0, COLORREF(0), 246, LWA_ALPHA)?;
    let dpi = GetDpiForWindow(hud.0);
    let s = |n| crate::capture_loupe_layout::scale(n, dpi);
    SetWindowPos(
        hud.0,
        Some(HWND_TOPMOST),
        info.rcWork.right - s(184),
        info.rcWork.top + s(12),
        s(172),
        s(36),
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    )?;
    Ok(hud)
}
pub(super) unsafe fn start(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    let seconds = match command {
        THREE => 3,
        FIVE => 5,
        _ => return Err(Error::new(E_INVALIDARG, "Invalid capture delay")),
    };
    if active() || !ready(state) {
        return Ok(());
    }
    let timer = NEXT
        .with(|n| {
            let v = n.get();
            n.set(v.checked_add(1)?);
            Some(v)
        })
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Capture timer identity exhausted"))?;
    let hud = badge(seconds)?;
    if !IsWindow(Some(hwnd)).as_bool() || !ready(state) || active() {
        return Ok(());
    }
    if SetTimer(Some(hwnd), timer, 250, None) == 0 {
        return Err(Error::from_win32());
    }
    ACTIVE.with_borrow_mut(|slot| {
        *slot = Some(Pending {
            host: hwnd,
            timer,
            deadline: Instant::now() + Duration::from_secs(seconds.into()),
            hud,
            shown: seconds,
        })
    });
    Ok(())
}

pub(super) unsafe fn dispatch(hwnd: HWND, state: &State, msg: u32, wp: WPARAM) -> bool {
    dispatch_with(hwnd, state, msg, wp, || capture_input::delayed(hwnd, state))
}
unsafe fn dispatch_with(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    capture: impl FnOnce() -> Result<()>,
) -> bool {
    if !matches!(
        msg,
        WM_DESTROY | WM_DISPLAYCHANGE | WM_QUERYENDSESSION | WM_TIMER
    ) {
        return false;
    }
    let matches = ACTIVE.with_borrow(|p| {
        p.as_ref()
            .is_some_and(|p| p.host == hwnd && (msg != WM_TIMER || p.timer == wp.0))
    });
    if !matches {
        return false;
    }
    if msg != WM_TIMER || state.exit_requested.get() || state.session_end.paused() {
        cancel();
        return msg == WM_TIMER;
    }
    let update = ACTIVE.with_borrow_mut(|p| {
        let p = p.as_mut()?;
        let remaining = p
            .deadline
            .saturating_duration_since(Instant::now())
            .as_nanos()
            .div_ceil(1_000_000_000) as u32;
        if remaining == 0 {
            return None;
        }
        let changed = p.shown != remaining;
        p.shown = remaining;
        Some(changed.then_some((p.hud.0, remaining)))
    });
    let result = match update {
        Some(Some((badge, n))) => SetWindowTextW(badge, &label(n)),
        Some(None) => Ok(()),
        None => {
            cancel(); // Destroy badge before the compositor fence, not just after selecting.
            if ready(state) {
                let _ = DwmFlush();
                capture()
            } else {
                Ok(())
            }
        }
    };
    if let Err(error) = result {
        cancel();
        if IsWindow(Some(hwnd)).as_bool() && !state.modals.blocked() && !state.exit_requested.get()
        {
            if let Some(_guard) = super::file_dialog::Guard::begin(&state.file_dialog) {
                MessageBoxW(
                    Some(hwnd),
                    &HSTRING::from(crate::ui_text::format(
                        "capture-failed",
                        &[("error", error.to_string().into())],
                    )),
                    w!("Otter"),
                    MB_OK | MB_ICONWARNING,
                );
            }
        }
    }
    true
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[test]
fn countdown_cancel_and_stale_ticks_never_capture_and_expiry_retires_hud_first() {
    unsafe {
        let owner = Window(
            CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("STATIC"),
                w!(""),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
            .unwrap(),
        );
        let state = State::new(None, false).unwrap();
        let read = || ACTIVE.with_borrow(|slot| slot.as_ref().map(|p| (p.timer, p.hud.0)));
        start(owner.0, &state, THREE).unwrap();
        let (old, hud) = read().unwrap();
        assert!(!state.capture_active.get());
        let style = GetWindowLongPtrW(hud, GWL_EXSTYLE) as u32;
        assert_eq!(
            style & (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0,
            (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0
        );
        let mut affinity = 0;
        GetWindowDisplayAffinity(hud, &mut affinity).unwrap();
        assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        start(owner.0, &state, FIVE).unwrap();
        assert_eq!(read(), Some((old, hud)));
        assert!(dispatch_with(
            owner.0,
            &state,
            WM_TIMER,
            WPARAM(old),
            || panic!("early capture")
        ));
        cancel();
        assert!(!IsWindow(Some(hud)).as_bool());
        start(owner.0, &state, FIVE).unwrap();
        assert!(!dispatch_with(
            owner.0,
            &state,
            WM_TIMER,
            WPARAM(old),
            || panic!("stale capture")
        ));
        dispatch_with(owner.0, &state, WM_DISPLAYCHANGE, WPARAM(0), || {
            panic!("invalidated")
        });
        assert!(!active());
        start(owner.0, &state, THREE).unwrap();
        let (id, hud) = read().unwrap();
        ACTIVE.with_borrow_mut(|slot| slot.as_mut().unwrap().deadline = Instant::now());
        let called = Cell::new(false);
        assert!(dispatch_with(owner.0, &state, WM_TIMER, WPARAM(id), || {
            assert!(!active() && !IsWindow(Some(hud)).as_bool());
            called.set(true);
            Ok(())
        }));
        assert!(called.get());
    }
}
