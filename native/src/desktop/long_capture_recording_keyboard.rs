//! Recording-only Enter/Esc routing; consume unmodified key edges only, without recording text or changing source-window focus.
use super::*;
use windows::Win32::{System::LibraryLoader::GetModuleHandleW, UI::Input::KeyboardAndMouse::*};
const MESSAGE: u32 = WM_APP + 0x5c2;
thread_local! {
    static DESTINATION: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
    static HELD: Cell<u8> = const { Cell::new(0) };
    static CONSUMED: Cell<u8> = const { Cell::new(0) };
    static TOKEN: Cell<usize> = const { Cell::new(0) };
}
pub(super) struct Hook {
    hook: HHOOK,
    token: usize,
}
impl Hook {
    pub unsafe fn install(hwnd: HWND) -> Result<Self> {
        if DESTINATION.with(|value| !value.get().is_invalid()) {
            return Err(Error::from(E_UNEXPECTED));
        }
        let token = TOKEN
            .with(Cell::get)
            .checked_add(1)
            .ok_or_else(|| Error::from(E_UNEXPECTED))?;
        let hook = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(callback),
            Some(GetModuleHandleW(None)?.into()),
            0,
        )?;
        TOKEN.with(|v| v.set(token));
        HELD.with(|v| {
            v.set(u8::from(GetAsyncKeyState(13) < 0) | (u8::from(GetAsyncKeyState(27) < 0) << 1))
        });
        CONSUMED.with(|v| v.set(0));
        DESTINATION.with(|v| v.set(hwnd));
        Ok(Self { hook, token })
    }
}
impl Drop for Hook {
    fn drop(&mut self) {
        if TOKEN.with(Cell::get) == self.token {
            DESTINATION.with(|v| v.set(HWND::default()));
            HELD.with(|v| v.set(0));
            CONSUMED.with(|v| v.set(0));
        }
        unsafe {
            let _ = UnhookWindowsHookEx(self.hook);
        }
    }
}
unsafe extern "system" fn callback(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let handled = std::panic::catch_unwind(|| {
        if code != HC_ACTION as i32 || lp.0 == 0 {
            return false;
        }
        let key = &*(lp.0 as *const KBDLLHOOKSTRUCT);
        if key.flags.contains(LLKHF_INJECTED) || !matches!(key.vkCode, 13 | 27) {
            return false;
        }
        let modifiers = matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN)
            && [VK_CONTROL, VK_SHIFT, VK_MENU, VK_LWIN, VK_RWIN]
                .iter()
                .any(|k| GetAsyncKeyState(k.0 as i32) < 0);
        route(key.vkCode, wp.0 as u32, modifiers)
    })
    .unwrap_or(false);
    if handled {
        LRESULT(1)
    } else {
        CallNextHookEx(None, code, wp, lp)
    }
}
unsafe fn route(key: u32, message: u32, modifiers: bool) -> bool {
    let bit = match key {
        13 => 1,
        27 => 2,
        _ => return false,
    };
    let hwnd = DESTINATION.with(Cell::get);
    if hwnd.is_invalid() || !IsWindow(Some(hwnd)).as_bool() {
        return false;
    }
    let held = HELD.with(Cell::get);
    let consumed = CONSUMED.with(Cell::get);
    if matches!(message, WM_KEYUP | WM_SYSKEYUP) {
        HELD.with(|v| v.set(held & !bit));
        CONSUMED.with(|v| v.set(consumed & !bit));
        return consumed & bit != 0;
    }
    if !matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) {
        return false;
    }
    HELD.with(|v| v.set(held | bit));
    if held & bit != 0 {
        return consumed & bit != 0;
    }
    if modifiers {
        return false;
    }
    if PostMessageW(
        Some(hwnd),
        MESSAGE,
        WPARAM(TOKEN.with(Cell::get)),
        LPARAM(key as isize),
    )
    .is_err()
    {
        return false;
    }
    CONSUMED.with(|v| v.set(consumed | bit));
    true
}
pub(super) unsafe fn dispatch(hwnd: HWND, input: &Input, message: &MSG) -> bool {
    if message.message != MESSAGE || message.hwnd != hwnd {
        return false;
    }
    if DESTINATION.with(Cell::get) == hwnd && TOKEN.with(Cell::get) == message.wParam.0 {
        match message.lParam.0 {
            13 => finish::request(hwnd, input),
            27 => close(hwnd, input),
            _ => {}
        }
    }
    true
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_bare_recording_keys_queue_one_edge_and_cleanup_stops_routing() {
        unsafe {
            let window = crate::desktop::Window(
                CreateWindowExW(
                    WS_EX_TOOLWINDOW,
                    w!("STATIC"),
                    w!("keys"),
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
            let hook = Hook::install(window.0).unwrap();
            assert!(!route(13, WM_KEYDOWN, true));
            assert!(!route(13, WM_KEYDOWN, false));
            assert!(!route(13, WM_KEYUP, false));
            assert!(!route(65, WM_KEYDOWN, false));
            for key in [13, 27] {
                assert!(route(key, WM_KEYDOWN, false));
                assert!(route(key, WM_KEYDOWN, false));
                assert!(route(key, WM_KEYDOWN, true));
                let mut message = MSG::default();
                assert!(
                    PeekMessageW(&mut message, Some(window.0), MESSAGE, MESSAGE, PM_REMOVE)
                        .as_bool()
                );
                assert_eq!(message.lParam.0, key as isize);
                assert!(
                    !PeekMessageW(&mut message, Some(window.0), MESSAGE, MESSAGE, PM_REMOVE)
                        .as_bool()
                );
                assert!(route(key, WM_KEYUP, false));
            }
            drop(hook);
            assert!(!route(13, WM_KEYDOWN, false));
        }
    }
}
