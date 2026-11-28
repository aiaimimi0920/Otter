//! Owned style bits for whole-pin input transparency. Mutating styles can
//! reenter the window procedure; verify identity before every further write.
use super::State;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows::Win32::{
    Graphics::{Dwm::DwmFlush, Gdi::*},
    UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture},
};
use windows::{
    core::{Error, Result},
    Win32::{Foundation::*, System::Threading::GetCurrentThreadId, UI::WindowsAndMessaging::*},
};

#[path = "pin_click_panels.rs"]
pub(super) mod panels;
pub(super) const RESTORE: usize = 0x21e2;
#[derive(Default)]
pub(super) struct Signals {
    pub enabled: Cell<bool>,
    pub count: Cell<usize>,
    pub all: Cell<bool>,
    pub toggle_all: Cell<bool>,
    pub pointer: Cell<Option<POINT>>,
    tray_attempts: Cell<u8>,
}
impl Signals {
    pub fn try_tray_recovery(&self, available: bool) -> bool {
        if available || (!self.enabled.get() && self.count.get() == 0) {
            self.tray_attempts.set(0);
        } else if self.tray_attempts.get() < 3 {
            self.tray_attempts.set(self.tray_attempts.get() + 1);
            return true;
        }
        false
    }
    pub fn retry_pending(&self) -> bool {
        (self.enabled.get() || self.count.get() > 0) && (1..3).contains(&self.tray_attempts.get())
    }
}
#[derive(Default)]
pub(super) struct Mode {
    pub signals: Rc<Signals>,
    identity: Rc<()>,
    session: RefCell<Option<Session>>,
    changing: Cell<bool>,
    retry_region: Cell<bool>,
    pub chrome: RefCell<Option<super::application::chrome::Suspended>>,
}
struct Session {
    style: Styles,
    toolbar: Option<(HWND, usize)>,
    settings: bool,
    surface: Rc<()>,
    stamp: crate::editor::DocumentStamp,
}
impl Mode {
    pub fn active(&self) -> bool {
        self.session.borrow().is_some()
    }
    pub fn retire(&self) {
        if self.session.take().is_some() {
            self.signals
                .count
                .set(self.signals.count.get().saturating_sub(1));
        }
    }
}

fn unavailable() -> Error {
    Error::new(E_FAIL, crate::ui_text::text("pin-pass-unavailable"))
}
pub(super) fn drafting(s: &State) -> bool {
    s.inline_text.borrow().is_some()
        || s.canvas
            .try_borrow()
            .map_or(true, |c| c.blocks_wheel() || c.drag.is_some())
}
fn owns(hwnd: HWND, s: &State) -> bool {
    s.lifetime.owns(hwnd)
        && unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } == s as *const State as isize
}

pub(super) unsafe fn enter(hwnd: HWND, s: &State) -> Result<()> {
    if !owns(hwnd, s)
        || !super::tray::recovery_installed(s)
        || s.click.active()
        || s.click.changing.get()
        || drafting(s)
        || !super::pin_settings::connection_popup_idle(s)
        || s.is_art()
        || s.live_pin.get()
        || super::pin_window::animation::moving(s)
        || super::group_lock::editing_busy(s)
        || s.group_hidden.get()
        || s.group_locked.get()
        || s.file_dialog.get()
        || s.capture_active.get()
        || s.session_end.paused()
        || !IsWindowVisible(hwnd).as_bool()
        || IsIconic(hwnd).as_bool()
    {
        return Err(unavailable());
    }
    if super::pin_window::preview::active(s) || super::pin_window::animation::zoom_busy(s) {
        super::pin_window::animation::stop(hwnd, s);
    }
    if !owns(hwnd, s) {
        return Err(unavailable());
    }
    let focus = windows::Win32::UI::Input::KeyboardAndMouse::GetFocus();
    if [
        panels::toolbar(s).map(|(h, _)| h),
        panels::property(s),
        super::pin_settings::window(s),
    ]
    .into_iter()
    .flatten()
    .any(|h| focus == h || IsChild(h, focus).as_bool())
    {
        let _ = windows::Win32::UI::Input::KeyboardAndMouse::SetFocus(None);
    }
    if !panels::idle(s) {
        return Err(unavailable());
    }
    let (stamp, surface) = {
        let c = s.canvas.borrow();
        if !c.pinned
            || c.crop_mode
            || c.minified.is_some()
            || c.image.is_none()
            || s.resolved_display.borrow().is_some()
        {
            return Err(unavailable());
        }
        (
            c.editor.document_stamp(),
            c.input_surface().ok_or_else(unavailable)?,
        )
    };
    let cap = GetCapture();
    if !cap.is_invalid() && cap != hwnd {
        return Err(unavailable());
    }
    let owner = || owns(hwnd, s);
    let style = Styles::snapshot(hwnd, &owner)?;
    let panel = panels::toolbar(s);
    if let Some(bar) = panels::property(s) {
        panels::bind(bar, hwnd, s, false)?;
    }
    if let Some(settings) = super::pin_settings::window(s) {
        panels::bind(settings, hwnd, s, false)?;
    }
    *s.click.chrome.borrow_mut() = super::application::chrome::Suspended::capture(s, hwnd);
    s.click.session.replace(Some(Session {
        style,
        toolbar: panel.filter(|(h, _)| IsWindowVisible(*h).as_bool()),
        settings: super::pin_settings::is_open(s),
        surface,
        stamp,
    }));
    s.click.signals.count.set(s.click.signals.count.get() + 1);
    s.click.changing.set(true);
    let result = (|| {
        if cap == hwnd {
            ReleaseCapture()?;
        }
        Styles::enable(hwnd, &owner)?;
        if !owner() {
            return Err(unavailable());
        }
        if SetWindowRgn(hwnd, None, false) == 0 {
            return Err(Error::from_win32());
        }
        if !owner() {
            return Err(unavailable());
        }
        s.hit_region.borrow_mut().invalidate();
        if super::pin_settings::is_open(s) {
            super::pin_settings::close(s);
        }
        if !owner() {
            return Err(unavailable());
        }
        if let Some((h, _)) = panel.filter(|p| Some(*p) == panels::toolbar(s)) {
            let _ = ShowWindow(h, SW_HIDE);
        }
        if !owner() || !super::tray::recovery_installed(s) {
            return Err(unavailable());
        }
        Ok(())
    })();
    s.click.changing.set(false);
    if let Err(error) = result {
        if owner() {
            leave(hwnd, s)?
        }
        return Err(error);
    }
    Ok(())
}
pub(super) unsafe fn confirm(hwnd: HWND, s: &State) -> Result<()> {
    if !owns(hwnd, s) || !s.click.active() || !super::tray::recovery_installed(s) {
        return Err(unavailable());
    }
    let _ = DwmFlush();
    Ok(())
}
pub(super) unsafe fn leave(hwnd: HWND, s: &State) -> Result<()> {
    if !owns(hwnd, s) || s.click.changing.get() {
        return Err(unavailable());
    }
    let Some((style, toolbar, settings)) = s.click.session.borrow().as_ref().map(|v| {
        (
            v.style,
            v.toolbar,
            v.settings && v.stamp.same_document(&s.canvas.borrow().editor),
        )
    }) else {
        return Ok(());
    };
    s.click.changing.set(true);
    let owner = || owns(hwnd, s);
    let result = (|| {
        style.restore(hwnd, &owner)?;
        s.hit_region.borrow_mut().invalidate();
        let rebuilt = super::hit_region::rebuild(hwnd, s);
        if !owner() {
            return Err(unavailable());
        }
        // Once styles are restored the window can receive input. Do not
        // leave keyboard and child windows locked out if region creation failed.
        s.click.retire();
        s.click.retry_region.set(rebuilt.is_err());
        if rebuilt.is_err() {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        let mut settings_result = Ok(());
        if IsWindowVisible(hwnd).as_bool()
            && !s.group_hidden.get()
            && !s.shake.hidden.get()
            && !super::tray::is_hidden(s)
        {
            if let Some((h, _)) = toolbar.filter(|p| Some(*p) == panels::toolbar(s)) {
                let _ = ShowWindow(h, SW_SHOWNOACTIVATE);
            }
            if settings && owner() && !super::pin_settings::is_open(s) {
                settings_result = super::pin_settings::toggle(hwnd, s);
            }
        }
        rebuilt.and(settings_result)
    })();
    s.click.changing.set(false);
    result
}

pub(super) fn input_result(s: &State, msg: u32, wp: WPARAM) -> Option<LRESULT> {
    if !s.click.active() {
        return None;
    }
    match msg {
        WM_NCHITTEST => Some(LRESULT(HTTRANSPARENT as isize)),
        WM_KEYFIRST..=WM_KEYLAST
        | WM_MOUSEFIRST..=WM_MOUSELAST
        | WM_NCMOUSEMOVE..=WM_NCXBUTTONDBLCLK
        | WM_IME_STARTCOMPOSITION
        | WM_IME_ENDCOMPOSITION
        | WM_IME_COMPOSITION
        | WM_CONTEXTMENU
        | WM_COMMAND
        | super::tool_panel::EXECUTE => Some(LRESULT(0)),
        WM_SYSCOMMAND if matches!(wp.0 as u32 & 0xfff0, SC_MOVE | SC_SIZE) => Some(LRESULT(0)),
        _ => None,
    }
}

pub(super) unsafe fn before_paint(hwnd: HWND, s: &State) -> Result<()> {
    if s.click.changing.get() {
        return Ok(());
    }
    if s.click.retry_region.replace(false) && !s.click.active() && owns(hwnd, s) {
        super::hit_region::rebuild(hwnd, s)?;
    }
    let changed = s.click.session.borrow().as_ref().is_some_and(|pass| {
        let c = s.canvas.borrow();
        !pass.stamp.same_document(&c.editor)
            || !c
                .input_surface()
                .is_some_and(|t| Rc::ptr_eq(&t, &pass.surface))
    });
    if changed {
        leave(hwnd, s)?;
    }
    Ok(())
}

#[cfg(test)]
mod recovery_tests {
    use super::Signals;
    #[test]
    fn missing_tray_bounds_wakes_and_rearms_when_available_or_empty() {
        let signals = Signals::default();
        signals.count.set(1);
        for pending in [true, true, false] {
            assert!(signals.try_tray_recovery(false));
            assert_eq!(signals.retry_pending(), pending);
        }
        assert!(!signals.try_tray_recovery(false));
        assert!(!signals.try_tray_recovery(true));
        assert!(signals.try_tray_recovery(false));
        signals.count.set(0);
        assert!(!signals.try_tray_recovery(false));
        assert!(!signals.retry_pending());
    }
}

const BITS: u32 = WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0;

#[derive(Clone, Copy)]
pub(super) struct Styles(u32);
impl Styles {
    pub unsafe fn snapshot(hwnd: HWND, owns: &impl Fn() -> bool) -> Result<Self> {
        Ok(Self(read(hwnd, owns)? & BITS))
    }
    pub unsafe fn enable(hwnd: HWND, owns: &impl Fn() -> bool) -> Result<()> {
        set(hwnd, BITS, owns)
    }
    pub unsafe fn restore(self, hwnd: HWND, owns: &impl Fn() -> bool) -> Result<()> {
        set(hwnd, self.0, owns)
    }
}

unsafe fn read(hwnd: HWND, owns: &impl Fn() -> bool) -> Result<u32> {
    if !owns() || GetWindowThreadProcessId(hwnd, None) != GetCurrentThreadId() {
        return Err(Error::new(E_HANDLE, "Pin input style owner changed"));
    }
    SetLastError(WIN32_ERROR(0));
    let result = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
    if result == 0 && GetLastError() != WIN32_ERROR(0) {
        return Err(Error::from_win32());
    }
    Ok(result as u32)
}
unsafe fn set(hwnd: HWND, bits: u32, owns: &impl Fn() -> bool) -> Result<()> {
    let before = read(hwnd, owns)?;
    let after = (before & !BITS) | bits;
    if before != after {
        SetLastError(WIN32_ERROR(0));
        if SetWindowLongPtrW(hwnd, GWL_EXSTYLE, after as isize) == 0
            && GetLastError() != WIN32_ERROR(0)
        {
            return Err(Error::from_win32());
        }
        read(hwnd, owns)?;
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
        )?;
    }
    if read(hwnd, owns)? & (BITS | WS_EX_NOREDIRECTIONBITMAP.0)
        != after & (BITS | WS_EX_NOREDIRECTIONBITMAP.0)
    {
        return Err(Error::new(E_FAIL, "Pin input style did not take effect"));
    }
    Ok(())
}
