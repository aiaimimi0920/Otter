//! An opt-in, event-driven foreground classification. Never infer a game from geometry alone.
use crate::shortcut_policy::Scope;
use std::cell::Cell;
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::Threading::GetCurrentProcessId,
        UI::{Accessibility::*, Shell::*, WindowsAndMessaging::*},
    },
};

pub(super) const MESSAGE: u32 = WM_APP + 0x342;
thread_local! {
    static DEST:Cell<Option<(HWND,bool)>>=const {Cell::new(None)};
}
pub(super) struct Watch {
    hwnd: HWND,
    hooks: Vec<HWINEVENTHOOK>,
}
impl Watch {
    pub(super) unsafe fn install(hwnd: HWND) -> Result<Self> {
        if DEST.with(Cell::get).is_some() {
            return Err(Error::new(E_UNEXPECTED, "Duplicate fullscreen owner"));
        }
        let mut watch = Self {
            hwnd,
            hooks: Vec::with_capacity(2),
        };
        for event in [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_LOCATIONCHANGE] {
            let hook = SetWinEventHook(
                event,
                event,
                None,
                Some(notification),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if hook.is_invalid() {
                return Err(Error::from_win32());
            }
            watch.hooks.push(hook);
        }
        DEST.with(|target| target.set(Some((hwnd, false))));
        Ok(watch)
    }
}
impl Drop for Watch {
    fn drop(&mut self) {
        DEST.with(|target| {
            if target.get().is_some_and(|(h, _)| h == self.hwnd) {
                target.set(None)
            }
        });
        for hook in self.hooks.drain(..) {
            unsafe {
                let _ = UnhookWinEvent(hook);
            }
        }
    }
}
unsafe extern "system" fn notification(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    child: i32,
    _: u32,
    _: u32,
) {
    if event == EVENT_OBJECT_LOCATIONCHANGE
        && (object != OBJID_WINDOW.0 || child != 0 || hwnd != GetForegroundWindow())
    {
        return;
    }
    DEST.with(|target| {
        if let Some((owner, false)) = target.get() {
            if PostMessageW(Some(owner), MESSAGE, WPARAM(0), LPARAM(0)).is_ok() {
                target.set(Some((owner, true)));
            }
        }
    });
}
pub(super) fn acknowledge(hwnd: HWND) {
    DEST.with(|target| {
        if target.get() == Some((hwnd, true)) {
            target.set(Some((hwnd, false)))
        }
    });
}
pub(super) unsafe fn matches(scope: Scope) -> bool {
    let hwnd = GetForegroundWindow();
    let mut pid = 0;
    if hwnd.is_invalid()
        || GetWindowThreadProcessId(hwnd, Some(&mut pid)) == 0
        || pid == GetCurrentProcessId()
        || !IsWindowVisible(hwnd).as_bool()
        || IsIconic(hwnd).as_bool()
    {
        return false;
    }
    let mut class = [0u16; 128];
    let n = GetClassNameW(hwnd, &mut class);
    if n <= 0
        || [
            "Progman",
            "WorkerW",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
        ]
        .iter()
        .any(|name| class[..n as usize].iter().copied().eq(name.encode_utf16()))
    {
        return false;
    }
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let mut rect = RECT::default();
    let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);
    if monitor.is_invalid()
        || !GetMonitorInfoW(monitor, &mut info).as_bool()
        || GetWindowRect(hwnd, &mut rect).is_err()
    {
        return false;
    }
    // A maximized work-area window can have invisible resize borders outside the
    // monitor. Require the actual client area, not those borders, to cover it.
    let mut client = RECT::default();
    if GetClientRect(hwnd, &mut client).is_err() {
        return false;
    }
    let mut origin = POINT::default();
    if !ClientToScreen(hwnd, &mut origin).as_bool() {
        return false;
    }
    let r = info.rcMonitor;
    origin.x <= r.left
        && origin.y <= r.top
        && origin.x.saturating_add(client.right) >= r.right
        && origin.y.saturating_add(client.bottom) >= r.bottom
        && (scope == Scope::AllFullscreen
            || SHQueryUserNotificationState().is_ok_and(|s| s == QUNS_RUNNING_D3D_FULL_SCREEN))
}
