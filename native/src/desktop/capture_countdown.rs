//! Region-first delayed capture on the workspace UI thread. Select bounds first, keep click-through frame + countdown visible during the delay, then lock pixels on expiry.
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
        UI::{HiDpi::GetDpiForWindow, Shell::*, WindowsAndMessaging::*},
    },
};

pub(super) const THREE: usize = 0x1ef1;
pub(super) const FIVE: usize = 0x1ef2;
const BADGE_SUBCLASS: usize = 0x434442;
const BORDER_SUBCLASS: usize = 0x434446;
const SIGNAL: COLORREF = COLORREF(0x38ffd9);
const PANEL: COLORREF = COLORREF(0x18120e);

struct Pending {
    host: HWND,
    timer: usize,
    deadline: Instant,
    region: RECT,
    hud: Window,
    _frame: [Window; 4],
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
    static ACTIVE: RefCell<Option<Pending>> = const { RefCell::new(None) };
    // Never reuse timer IDs: KillTimer does not remove already queued WM_TIMER messages.
    static NEXT: Cell<usize> = const { Cell::new(0x33000000) };
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

unsafe extern "system" fn border_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            if !dc.is_invalid() {
                let mut client = RECT::default();
                let _ = GetClientRect(hwnd, &mut client);
                SetDCBrushColor(dc, SIGNAL);
                FillRect(dc, &client, HBRUSH(GetStockObject(DC_BRUSH).0));
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let _ = RemoveWindowSubclass(hwnd, Some(border_proc), id);
            DefSubclassProc(hwnd, msg, wp, lp)
        }
        _ => DefSubclassProc(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn badge_proc(
    hwnd: HWND,
    msg: u32,
    wp: WPARAM,
    lp: LPARAM,
    id: usize,
    _: usize,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let dc = BeginPaint(hwnd, &mut ps);
            if !dc.is_invalid() {
                let mut client = RECT::default();
                let _ = GetClientRect(hwnd, &mut client);
                let brush = HBRUSH(GetStockObject(DC_BRUSH).0);
                SetDCBrushColor(dc, PANEL);
                FillRect(dc, &client, brush);
                SetDCBrushColor(dc, SIGNAL);
                FrameRect(dc, &client, brush);
                let dpi = GetDpiForWindow(hwnd).max(96);
                let font_h = -crate::capture_loupe_layout::scale(13, dpi).max(12);
                let custom_font = CreateFontW(
                    font_h,
                    0,
                    0,
                    0,
                    FW_SEMIBOLD.0 as i32,
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    0,
                    w!("Microsoft YaHei UI"),
                );
                let font_obj = if custom_font.is_invalid() {
                    GetStockObject(DEFAULT_GUI_FONT)
                } else {
                    HGDIOBJ(custom_font.0)
                };
                let old = SelectObject(dc, font_obj);
                SetBkMode(dc, TRANSPARENT);
                SetTextColor(dc, SIGNAL);
                let mut buf = [0u16; 128];
                let len = GetWindowTextW(hwnd, &mut buf) as usize;
                let mut text_rect = client;
                DrawTextW(
                    dc,
                    &mut buf[..len],
                    &mut text_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
                );
                let _ = SelectObject(dc, old);
                if !custom_font.is_invalid() {
                    let _ = DeleteObject(HGDIOBJ(custom_font.0));
                }
            }
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let _ = RemoveWindowSubclass(hwnd, Some(badge_proc), id);
            DefSubclassProc(hwnd, msg, wp, lp)
        }
        _ => DefSubclassProc(hwnd, msg, wp, lp),
    }
}

unsafe fn overlay_window(text: &HSTRING) -> Result<Window> {
    let w = Window(CreateWindowExW(
        WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
        w!("STATIC"),
        text,
        WS_DISABLED | WS_POPUP,
        0,
        0,
        1,
        1,
        None,
        None,
        None,
        None,
    )?);
    SetPropW(w.0, w!("NonRudeHWND"), Some(HANDLE(1usize as *mut _)))?;
    SetWindowDisplayAffinity(w.0, WDA_EXCLUDEFROMCAPTURE)?;
    Ok(w)
}

unsafe fn build_overlay(region: RECT, seconds: u32) -> Result<(Window, [Window; 4])> {
    if region.right <= region.left || region.bottom <= region.top {
        return Err(Error::new(E_INVALIDARG, "Invalid delayed capture region"));
    }
    let center = POINT {
        x: region.left + (region.right - region.left) / 2,
        y: region.top + (region.bottom - region.top) / 2,
    };
    let monitor = MonitorFromPoint(center, MONITOR_DEFAULTTONEAREST);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as _,
        ..Default::default()
    };
    GetMonitorInfoW(monitor, &mut info).ok()?;
    let hud = overlay_window(&label(seconds))?;
    SetLayeredWindowAttributes(hud.0, COLORREF(0), 242, LWA_ALPHA)?;
    SetWindowSubclass(hud.0, Some(badge_proc), BADGE_SUBCLASS, 0).ok()?;
    let dpi = GetDpiForWindow(hud.0).max(96);
    let s = |n| crate::capture_loupe_layout::scale(n, dpi);
    let bw = s(196);
    let bh = s(30);
    let bx = region.left.clamp(
        info.rcWork.left + s(4),
        (info.rcWork.right - bw - s(4)).max(info.rcWork.left),
    );
    let by = if region.top - bh - s(6) >= info.rcWork.top {
        region.top - bh - s(6)
    } else {
        (region.top + s(6)).min((info.rcWork.bottom - bh).max(info.rcWork.top))
    };
    let line = s(2).max(2);
    let strips = [
        (
            region.left.saturating_sub(line),
            region.top.saturating_sub(line),
            region.right.saturating_add(line),
            region.top,
        ),
        (
            region.left.saturating_sub(line),
            region.bottom,
            region.right.saturating_add(line),
            region.bottom.saturating_add(line),
        ),
        (
            region.left.saturating_sub(line),
            region.top,
            region.left,
            region.bottom,
        ),
        (
            region.right,
            region.top,
            region.right.saturating_add(line),
            region.bottom,
        ),
    ];
    let empty = HSTRING::new();
    let mut frame_vec = Vec::with_capacity(4);
    for (l, t, r, b) in strips {
        let strip = overlay_window(&empty)?;
        SetLayeredWindowAttributes(strip.0, COLORREF(0), 240, LWA_ALPHA)?;
        SetWindowSubclass(strip.0, Some(border_proc), BORDER_SUBCLASS, 0).ok()?;
        SetWindowPos(
            strip.0,
            Some(HWND_TOPMOST),
            l,
            t,
            (r - l).max(1),
            (b - t).max(1),
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )?;
        frame_vec.push(strip);
    }
    let frame: [Window; 4] = frame_vec
        .try_into()
        .map_err(|_| Error::new(E_FAIL, "Incomplete delayed capture outline"))?;
    SetWindowPos(
        hud.0,
        Some(HWND_TOPMOST),
        bx,
        by,
        bw,
        bh,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    )?;
    Ok((hud, frame))
}

pub(super) unsafe fn start(hwnd: HWND, state: &State, command: usize) -> Result<()> {
    let seconds = match command {
        THREE => 3,
        FIVE => 5,
        _ => return Err(Error::new(E_INVALIDARG, "Invalid capture delay")),
    };
    if active() || !ready(state) || !capture_input::exclusive_available(state) {
        return Ok(());
    }
    let Some(region) = capture_input::select_delay_region(hwnd, state)? else {
        return Ok(());
    };
    start_region(hwnd, state, seconds, region)
}

unsafe fn start_region(hwnd: HWND, state: &State, seconds: u32, region: RECT) -> Result<()> {
    if active() || !ready(state) || !IsWindow(Some(hwnd)).as_bool() {
        return Ok(());
    }
    let timer = NEXT
        .with(|n| {
            let v = n.get();
            n.set(v.checked_add(1)?);
            Some(v)
        })
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Capture timer identity exhausted"))?;
    let (hud, frame) = build_overlay(region, seconds)?;
    if !IsWindow(Some(hwnd)).as_bool() || !ready(state) || active() {
        return Ok(());
    }
    if SetTimer(Some(hwnd), timer, 200, None) == 0 {
        return Err(Error::from_win32());
    }
    ACTIVE.with_borrow_mut(|slot| {
        *slot = Some(Pending {
            host: hwnd,
            timer,
            deadline: Instant::now() + Duration::from_secs(seconds.into()),
            region,
            hud,
            _frame: frame,
            shown: seconds,
        })
    });
    Ok(())
}

pub(super) unsafe fn dispatch(hwnd: HWND, state: &State, msg: u32, wp: WPARAM) -> bool {
    dispatch_with(hwnd, state, msg, wp, |region| {
        capture_input::delayed(hwnd, state, region)
    })
}
unsafe fn dispatch_with(
    hwnd: HWND,
    state: &State,
    msg: u32,
    wp: WPARAM,
    capture: impl FnOnce(RECT) -> Result<()>,
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
    enum Step {
        Tick(Option<(HWND, u32)>),
        Fire(RECT),
    }
    let step = ACTIVE.with_borrow_mut(|p| {
        let p = p.as_mut()?;
        let remaining = p
            .deadline
            .saturating_duration_since(Instant::now())
            .as_nanos()
            .div_ceil(1_000_000_000) as u32;
        if remaining == 0 {
            return Some(Step::Fire(p.region));
        }
        let changed = p.shown != remaining;
        p.shown = remaining;
        Some(Step::Tick(changed.then_some((p.hud.0, remaining))))
    });
    let result = match step {
        Some(Step::Tick(Some((badge, n)))) => {
            let res = SetWindowTextW(badge, &label(n));
            let _ = InvalidateRect(Some(badge), None, false);
            res
        }
        Some(Step::Tick(None)) | None => Ok(()),
        Some(Step::Fire(region)) => {
            cancel(); // Destroy frame and badge before compositor fence so neither appears in captured pixels.
            if ready(state) {
                let _ = DwmFlush();
                capture(region)
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
        let target = RECT {
            left: 24,
            top: 48,
            right: 184,
            bottom: 148,
        };
        let read = || {
            ACTIVE.with_borrow(|slot| {
                slot.as_ref()
                    .map(|p| (p.timer, p.hud.0, p._frame[0].0, p.region))
            })
        };
        start_region(owner.0, &state, 3, target).unwrap();
        let (old, hud, border, saved_rect) = read().unwrap();
        assert_eq!(saved_rect, target);
        assert!(!state.capture_active.get());
        for w in [hud, border] {
            let style = GetWindowLongPtrW(w, GWL_EXSTYLE) as u32;
            assert_eq!(
                style & (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0,
                (WS_EX_NOACTIVATE | WS_EX_TRANSPARENT).0
            );
            let mut affinity = 0;
            GetWindowDisplayAffinity(w, &mut affinity).unwrap();
            assert_eq!(affinity, WDA_EXCLUDEFROMCAPTURE.0);
        }
        start_region(owner.0, &state, 5, target).unwrap();
        assert_eq!(read(), Some((old, hud, border, target)));
        assert!(dispatch_with(
            owner.0,
            &state,
            WM_TIMER,
            WPARAM(old),
            |_| panic!("early capture")
        ));
        cancel();
        assert!(!IsWindow(Some(hud)).as_bool() && !IsWindow(Some(border)).as_bool());
        start_region(owner.0, &state, 5, target).unwrap();
        assert!(!dispatch_with(
            owner.0,
            &state,
            WM_TIMER,
            WPARAM(old),
            |_| panic!("stale capture")
        ));
        dispatch_with(owner.0, &state, WM_DISPLAYCHANGE, WPARAM(0), |_| {
            panic!("invalidated")
        });
        assert!(!active());
        start_region(owner.0, &state, 3, target).unwrap();
        let (id, hud, border, _) = read().unwrap();
        ACTIVE.with_borrow_mut(|slot| slot.as_mut().unwrap().deadline = Instant::now());
        let called = Cell::new(None);
        assert!(dispatch_with(
            owner.0,
            &state,
            WM_TIMER,
            WPARAM(id),
            |rect| {
                assert!(
                    !active()
                        && !IsWindow(Some(hud)).as_bool()
                        && !IsWindow(Some(border)).as_bool()
                );
                called.set(Some(rect));
                Ok(())
            }
        ));
        assert_eq!(called.get(), Some(target));
    }
}
