//! 仅显式取色期间安装的鼠标 hook；回调不采样/绘制/分配，移动消息合并为一个待处理项。
use super::*;
pub(super) const MOVED: u32 = WM_APP + 0x241;
pub(super) const COMMIT: u32 = WM_APP + 0x242;
pub(super) const CANCEL: u32 = WM_APP + 0x243;
thread_local! {
    static TARGET: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
    static POSITION: Cell<POINT> = const { Cell::new(POINT { x: 0, y: 0 }) };
    static COMMITTED: Cell<POINT> = const { Cell::new(POINT { x: 0, y: 0 }) };
    static QUEUED: Cell<bool> = const { Cell::new(false) };
    static ENDING: Cell<bool> = const { Cell::new(false) };
    static GENERATION: Cell<usize> = const { Cell::new(0) };
}
pub(super) struct Hook(HHOOK);
impl Hook {
    pub(super) unsafe fn install(hwnd: HWND) -> Result<Self> {
        if TARGET.with(|target| !target.get().0.is_null()) {
            return Err(Error::new(E_FAIL, "Screen color hook already active"));
        }
        let module = windows::Win32::System::LibraryLoader::GetModuleHandleW(None)?;
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(callback), Some(module.into()), 0)?;
        TARGET.with(|target| target.set(hwnd));
        GENERATION.with(|value| value.set(value.get().wrapping_add(1).max(1)));
        QUEUED.with(|queued| queued.set(false));
        ENDING.with(|ending| ending.set(false));
        Ok(Self(hook))
    }
}
impl Drop for Hook {
    fn drop(&mut self) {
        TARGET.with(|target| target.set(HWND::default()));
        QUEUED.with(|queued| queued.set(false));
        ENDING.with(|ending| ending.set(false));
        unsafe {
            let _ = UnhookWindowsHookEx(self.0);
        }
    }
}
pub(super) fn take_move() -> POINT {
    QUEUED.with(|queued| queued.set(false));
    POSITION.with(Cell::get)
}
pub(super) fn commit_point() -> POINT {
    COMMITTED.with(Cell::get)
}
pub(super) fn is_current(token: usize) -> bool {
    token != 0
        && GENERATION.with(|value| value.get() == token)
        && TARGET.with(|target| !target.get().0.is_null())
}
unsafe extern "system" fn callback(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && lp.0 != 0 {
        let blocked = std::panic::catch_unwind(|| {
            forward(wp.0 as u32, (*(lp.0 as *const MSLLHOOKSTRUCT)).pt)
        })
        .unwrap_or(false);
        if blocked {
            return LRESULT(1);
        }
    }
    CallNextHookEx(None, code, wp, lp)
}
unsafe fn forward(message: u32, point: POINT) -> bool {
    let hwnd = TARGET.with(Cell::get);
    let token = GENERATION.with(Cell::get);
    if hwnd.0.is_null() {
        return false;
    }
    match message {
        WM_MOUSEMOVE if !ENDING.with(Cell::get) => {
            POSITION.with(|value| value.set(point));
            if !QUEUED.with(|value| value.replace(true))
                && PostMessageW(Some(hwnd), MOVED, WPARAM(token), LPARAM(0)).is_err()
            {
                QUEUED.with(|value| value.set(false));
            }
            false
        }
        WM_LBUTTONUP | WM_RBUTTONUP => {
            if !ENDING.with(|ending| ending.replace(true)) {
                COMMITTED.with(|value| value.set(point));
                let command = if message == WM_LBUTTONUP {
                    COMMIT
                } else {
                    CANCEL
                };
                if PostMessageW(Some(hwnd), command, WPARAM(token), LPARAM(0)).is_err() {
                    ENDING.with(|value| value.set(false));
                }
            }
            true
        }
        WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MOUSEWHEEL | WM_MOUSEHWHEEL => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Target(HWND);
    impl Drop for Target {
        fn drop(&mut self) {
            unsafe {
                TARGET.with(|value| value.set(HWND::default()));
                QUEUED.with(|value| value.set(false));
                ENDING.with(|value| value.set(false));
                let mut message = MSG::default();
                while PeekMessageW(&mut message, Some(self.0), MOVED, CANCEL, PM_REMOVE).as_bool() {
                }
                let _ = DestroyWindow(self.0);
            }
        }
    }
    #[test]
    fn synthetic_events_coalesce_preserve_physical_coordinates_and_reject_old_generation() {
        unsafe {
            let owner = Target(
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("hook routing test"),
                    WS_OVERLAPPED,
                    0,
                    0,
                    10,
                    10,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap(),
            );
            // 仅配置隐藏消息目标，绝不调用 SetWindowsHookEx 或发送真实键鼠。
            TARGET.with(|value| value.set(owner.0));
            GENERATION.with(|value| value.set(7));
            QUEUED.with(|value| value.set(false));
            ENDING.with(|value| value.set(false));
            for i in 0..1000 {
                assert!(!forward(WM_MOUSEMOVE, POINT { x: i, y: -i }));
            }
            let mut message = MSG::default();
            assert!(PeekMessageW(&mut message, Some(owner.0), MOVED, CANCEL, PM_REMOVE).as_bool());
            assert_eq!(message.message, MOVED);
            assert_eq!(message.wParam.0, 7);
            assert!(!PeekMessageW(&mut message, Some(owner.0), MOVED, CANCEL, PM_REMOVE).as_bool());
            let point = take_move();
            assert_eq!((point.x, point.y), (999, -999));
            assert!(forward(WM_LBUTTONDOWN, POINT::default()));
            assert!(forward(
                WM_LBUTTONUP,
                POINT {
                    x: -100_000,
                    y: 200_000
                }
            ));
            assert!(forward(WM_LBUTTONUP, POINT::default()));
            let point = commit_point();
            assert_eq!((point.x, point.y), (-100_000, 200_000));
            assert!(PeekMessageW(&mut message, Some(owner.0), MOVED, CANCEL, PM_REMOVE).as_bool());
            assert_eq!(message.message, COMMIT);
            assert!(!PeekMessageW(&mut message, Some(owner.0), MOVED, CANCEL, PM_REMOVE).as_bool());
            assert!(is_current(7));
            GENERATION.with(|value| value.set(8));
            assert!(!is_current(7));
            assert!(is_current(8));
        }
        assert!(!is_current(8));
    }
}
