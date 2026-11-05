//! 需求期 UI 线程 hooks；回调仅更新有界手势状态并置事件，不恢复窗口、不进行协议 I/O。
#[path = "live_reclaim_input.rs"]
mod input;
use crate::live_reclaim::Intent;
use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    os::windows::io::{AsHandle, AsRawHandle, BorrowedHandle, FromRawHandle, OwnedHandle},
    rc::Rc,
};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::*,
        System::{
            LibraryLoader::GetModuleHandleW,
            SystemInformation::GetTickCount64,
            Threading::{CreateEventW, ResetEvent, SetEvent},
        },
        UI::{Accessibility::*, WindowsAndMessaging::*},
    },
};

thread_local! {
    static INPUT: RefCell<input::Input> = RefCell::new(input::Input::default());
    static EVENT: Cell<HANDLE> = const { Cell::new(HANDLE(std::ptr::null_mut())) };
    static FAULT: Cell<bool> = const { Cell::new(false) };
}
pub struct Watcher {
    keyboard: Option<HHOOK>,
    mouse: Option<HHOOK>,
    foreground: HWINEVENTHOOK,
    minimize: HWINEVENTHOOK,
    event: OwnedHandle,
    _thread: PhantomData<Rc<()>>,
}
impl Watcher {
    pub fn install() -> Result<Self> {
        if EVENT.with(|event| !event.get().is_invalid()) {
            return Err(Error::new(
                E_UNEXPECTED,
                "Live reclaim watcher already installed",
            ));
        }
        let raw = unsafe { CreateEventW(None, true, false, None)? };
        let mut owner = Self {
            keyboard: None,
            mouse: None,
            foreground: HWINEVENTHOOK::default(),
            minimize: HWINEVENTHOOK::default(),
            event: unsafe { OwnedHandle::from_raw_handle(raw.0) },
            _thread: PhantomData,
        };
        INPUT.with(|input| *input.borrow_mut() = input::Input::default());
        FAULT.with(|fault| fault.set(false));
        EVENT.with(|event| event.set(raw));
        unsafe {
            let module = GetModuleHandleW(None)?;
            owner.keyboard = Some(SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard),
                Some(module.into()),
                0,
            )?);
            owner.mouse = Some(SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(mouse),
                Some(module.into()),
                0,
            )?);
            owner.foreground = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(window_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if owner.foreground.is_invalid() {
                return Err(Error::from_win32());
            }
            owner.minimize = SetWinEventHook(
                EVENT_SYSTEM_MINIMIZESTART,
                EVENT_SYSTEM_MINIMIZEEND,
                None,
                Some(window_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            if owner.minimize.is_invalid() {
                return Err(Error::from_win32());
            }
        }
        Ok(owner)
    }
    pub fn handle(&self) -> BorrowedHandle<'_> {
        self.event.as_handle()
    }
    pub fn snapshot(&self) -> Result<(usize, Option<Intent>, u64)> {
        unsafe {
            ResetEvent(HANDLE(self.event.as_raw_handle()))?;
        }
        if FAULT.with(Cell::get) {
            return Err(Error::new(E_FAIL, "Live reclaim watcher failed"));
        }
        let now = unsafe { GetTickCount64() };
        let current = INPUT
            .with(|input| input.try_borrow().map(|value| value.snapshot(now)))
            .map_err(|_| Error::new(E_FAIL, "Live reclaim watcher reentered"))?;
        Ok((unsafe { GetForegroundWindow() }.0 as usize, current, now))
    }
}
impl Drop for Watcher {
    fn drop(&mut self) {
        // 先撤销回调可见的事件，再注销 hook，最后 OwnedHandle 关闭事件。
        EVENT.with(|event| event.set(HANDLE::default()));
        unsafe {
            if let Some(hook) = self.keyboard {
                let _ = UnhookWindowsHookEx(hook);
            }
            if let Some(hook) = self.mouse {
                let _ = UnhookWindowsHookEx(hook);
            }
            if !self.foreground.is_invalid() {
                let _ = UnhookWinEvent(self.foreground);
            }
            if !self.minimize.is_invalid() {
                let _ = UnhookWinEvent(self.minimize);
            }
        }
    }
}
fn wake() {
    EVENT.with(|event| {
        let handle = event.get();
        if !handle.is_invalid() && unsafe { SetEvent(handle) }.is_err() {
            FAULT.with(|fault| fault.set(true));
        }
    });
}
fn update(action: impl FnOnce(&mut input::Input)) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        INPUT.with(|input| match input.try_borrow_mut() {
            Ok(mut input) => action(&mut input),
            Err(_) => FAULT.with(|fault| fault.set(true)),
        });
    }))
    .is_err()
    {
        FAULT.with(|fault| fault.set(true));
    }
    wake();
}
unsafe extern "system" fn keyboard(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && lp.0 != 0 {
        let data = &*(lp.0 as *const KBDLLHOOKSTRUCT);
        if data.flags.0 & (0x10 | 0x02) == 0 {
            let down = matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let key = data.vkCode;
            let now = GetTickCount64();
            let origin = GetForegroundWindow().0 as usize;
            update(|input| input.keyboard(key, down, false, now, origin));
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
unsafe extern "system" fn mouse(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32
        && lp.0 != 0
        && matches!(
            wp.0 as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        let data = &*(lp.0 as *const MSLLHOOKSTRUCT);
        if data.flags & 3 == 0 {
            let root = GetAncestor(WindowFromPoint(data.pt), GA_ROOT);
            let mut class = [0u16; 64];
            let length = GetClassNameW(root, &mut class).max(0) as usize;
            let shell = wp.0 as u32 == WM_LBUTTONDOWN && input::shell_class(&class[..length]);
            let now = GetTickCount64();
            let origin = GetForegroundWindow().0 as usize;
            update(|input| input.mouse_down(shell, false, now, origin));
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
unsafe extern "system" fn window_event(
    _: HWINEVENTHOOK,
    _: u32,
    _: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    wake();
}

#[cfg(test)]
#[path = "live_reclaim_watch_tests.rs"]
mod tests;
