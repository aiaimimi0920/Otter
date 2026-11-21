//! 仅接管指针命中的自有 Live 滚轮；不依赖 Windows 前台/后台滚轮策略，不重复投递。
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
thread_local! {
    static ROUTER: RefCell<Router> = RefCell::new(Router { hook: HHOOK::default(), windows: Vec::new() });
    #[cfg(test)]
    static ROUTED: Cell<usize> = const { Cell::new(0) };
}
#[cfg(test)]
pub(super) fn routed() -> usize {
    ROUTED.get()
}
struct Router {
    hook: HHOOK,
    windows: Vec<(HWND, isize)>,
}
pub(super) struct Registration(HWND);
impl Registration {
    pub(super) unsafe fn new(hwnd: HWND) -> Result<Self> {
        ROUTER.with(|owner| {
            let mut owner = owner.borrow_mut();
            owner
                .windows
                .try_reserve(1)
                .map_err(|_| Error::new(E_OUTOFMEMORY, "Live wheel registrations exhausted"))?;
            if owner.windows.is_empty() {
                owner.hook = SetWindowsHookExW(WH_MOUSE_LL, Some(callback), None, 0)?;
            }
            owner
                .windows
                .push((hwnd, GetWindowLongPtrW(hwnd, GWLP_USERDATA)));
            Ok(Self(hwnd))
        })
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        ROUTER.with(|owner| {
            let mut owner = owner.borrow_mut();
            owner.windows.retain(|(hwnd, _)| *hwnd != self.0);
            if owner.windows.is_empty() && !owner.hook.is_invalid() {
                unsafe {
                    let _ = UnhookWindowsHookEx(owner.hook);
                }
                owner.hook = HHOOK::default();
            }
        });
    }
}
unsafe extern "system" fn callback(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let handled = std::panic::catch_unwind(|| {
        if code != HC_ACTION as i32 || !matches!(wp.0 as u32, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) {
            return false;
        }
        let mouse = &*(lp.0 as *const MSLLHOOKSTRUCT);
        // Windows 按注册回调的默认上下文调用 hook；显式保持命中和投递坐标为物理像素。
        let _dpi = Dpi(SetThreadDpiAwarenessContext(
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        ));
        // LL hook 给物理屏幕坐标；回调 DPI 上下文可能不同于创建 Live 的 UI 上下文。
        let hwnd = WindowFromPhysicalPoint(mouse.pt);
        let marker = ROUTER.with(|owner| {
            owner
                .borrow()
                .windows
                .iter()
                .find(|(h, _)| *h == hwnd)
                .map(|(_, m)| *m)
        });
        let Some(marker) = marker
            .filter(|marker| *marker != 0 && GetWindowLongPtrW(hwnd, GWLP_USERDATA) == *marker)
        else {
            return false;
        };
        let state = &*(marker as *const State);
        #[cfg(test)]
        eprintln!("native-live-wheel-route: hwnd={hwnd:?}");
        if state.closed.get()
            || state.menu_open.get()
            || state.group_locked.get()
            || state.group_hidden.get()
            || state.session_end_reason.get()
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
