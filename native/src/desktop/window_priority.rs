//! 前景/窗口事件驱动的钉图让位；回调只合并通知，不遍历窗口或持有模型借用。
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        UI::{Accessibility::*, Input::KeyboardAndMouse::IsWindowEnabled, WindowsAndMessaging::*},
    },
};

pub(super) const PRIORITY_CHANGED: u32 = WM_APP + 44;
thread_local! {
    static DESTINATIONS: RefCell<Vec<(isize, bool)>> = const { RefCell::new(Vec::new()) };
    static SHARED: RefCell<Weak<Hooks>> = const { RefCell::new(Weak::new()) };
}

pub(super) struct Watcher {
    destination: isize,
    // Rc 同时保证共享订阅存活以及 !Send/!Sync，最后一个目标释放时注销系统 hook。
    _hooks: Rc<Hooks>,
}
struct Hooks {
    hooks: Vec<HWINEVENTHOOK>,
}

impl Watcher {
    pub fn install(hwnd: HWND) -> Result<Self> {
        if !unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            return Err(Error::new(E_INVALIDARG, "Invalid watcher destination"));
        }
        let destination = hwnd.0 as isize;
        if DESTINATIONS.with(|targets| {
            targets
                .borrow()
                .iter()
                .any(|target| target.0 == destination)
        }) {
            return Err(Error::new(
                E_UNEXPECTED,
                "Foreground watcher already installed for this window",
            ));
        }
        let existing = SHARED.with(|shared| shared.borrow().upgrade());
        let hooks = match existing {
            Some(hooks) => hooks,
            None => {
                let hooks = Rc::new(Hooks::install()?);
                SHARED.with(|shared| *shared.borrow_mut() = Rc::downgrade(&hooks));
                hooks
            }
        };
        DESTINATIONS.with(|targets| -> Result<()> {
            let mut targets = targets.borrow_mut();
            // 有界事件扇出，不因增加窗口而建立更多系统订阅或无界消息队列。
            if targets.len() >= 512 {
                return Err(Error::new(
                    E_OUTOFMEMORY,
                    "Foreground watcher destination budget exhausted",
                ));
            }
            targets
                .try_reserve(1)
                .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot allocate watcher destination"))?;
            targets.push((destination, false));
            Ok(())
        })?;
        Ok(Self {
            destination,
            _hooks: hooks,
        })
    }
}

impl Hooks {
    fn install() -> Result<Self> {
        let mut owner = Self {
            hooks: Vec::with_capacity(2),
        };
        for (first, last) in [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE),
        ] {
            let hook = unsafe {
                SetWinEventHook(
                    first,
                    last,
                    None,
                    Some(on_event),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT,
                )
            };
            if hook.is_invalid() {
                return Err(Error::from_win32());
            }
            owner.hooks.push(hook);
        }
        Ok(owner)
    }
}

#[cfg(test)]
#[path = "window_priority_tests.rs"]
mod tests;

impl Drop for Watcher {
    fn drop(&mut self) {
        DESTINATIONS.with(|targets| {
            let mut targets = targets.borrow_mut();
            targets.retain(|target| target.0 != self.destination);
            if targets.is_empty() {
                *targets = Vec::new();
            }
        });
    }
}

impl Drop for Hooks {
    fn drop(&mut self) {
        for hook in &self.hooks {
            unsafe {
                let _ = UnhookWinEvent(*hook);
            }
        }
    }
}

unsafe extern "system" fn on_event(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    object: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if event != EVENT_SYSTEM_FOREGROUND && object != OBJID_WINDOW.0 {
        return;
    }
    DESTINATIONS.with(|targets| {
        for (target, pending) in targets.borrow_mut().iter_mut() {
            if *pending || (event != EVENT_SYSTEM_FOREGROUND && hwnd.0 as isize == *target) {
                continue;
            }
            // PostMessage 不同步调用目标过程；只成功投递才设置该窗口的合并标志。
            *pending = PostMessageW(
                Some(HWND(*target as *mut _)),
                PRIORITY_CHANGED,
                WPARAM(0),
                LPARAM(0),
            )
            .is_ok();
        }
    });
}

pub(super) fn acknowledge(hwnd: HWND) {
    DESTINATIONS.with(|targets| {
        if let Some((_, pending)) = targets
            .borrow_mut()
            .iter_mut()
            .find(|target| target.0 == hwnd.0 as isize)
        {
            *pending = false;
        }
    });
}

pub(super) unsafe fn is_dialog(hwnd: HWND) -> bool {
    if !IsWindow(Some(hwnd)).as_bool() || !IsWindowVisible(hwnd).as_bool() {
        return false;
    }
    let mut class = [0u16; 128];
    let length = GetClassNameW(hwnd, &mut class).max(0) as usize;
    let standard = class[..length].iter().copied().eq("#32770".encode_utf16());
    let modal = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_DLGMODALFRAME.0 as isize != 0;
    let disabled_owner = GetWindow(hwnd, GW_OWNER)
        .ok()
        .is_some_and(|owner| !owner.is_invalid() && !IsWindowEnabled(owner).as_bool());
    standard || modal || disabled_owner
}

pub(super) unsafe fn blocker(pin: HWND) -> Option<HWND> {
    let foreground = GetForegroundWindow();
    let mut pid = 0;
    GetWindowThreadProcessId(foreground, Some(&mut pid));
    if foreground.is_invalid()
        || pid == 0
        || pid == std::process::id()
        || !IsWindowVisible(foreground).as_bool()
    {
        return None;
    }
    if is_dialog(foreground) || above(pin, foreground) {
        Some(foreground)
    } else {
        None
    }
}

unsafe fn above(pin: HWND, foreground: HWND) -> bool {
    let mut current = pin;
    // 有界 Z-order 遍历；不从任意 HWND 发起同步跨进程消息。
    for _ in 0..512 {
        let Ok(next) = GetWindow(current, GW_HWNDPREV) else {
            break;
        };
        if next.is_invalid() {
            break;
        }
        if next == foreground {
            return true;
        }
        current = next;
    }
    false
}
