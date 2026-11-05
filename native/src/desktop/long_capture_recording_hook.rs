//! 只在显式录制期存在的只读 wheel hook。回调仅过滤与覆盖固定大小提示，始终传递原输入。
use std::{cell::Cell, marker::PhantomData, rc::Rc};
use windows::{
    core::{w, Error, Result},
    Win32::{
        Foundation::*,
        System::{LibraryLoader::GetModuleHandleW, SystemInformation::GetTickCount64},
        UI::WindowsAndMessaging::*,
    },
};
pub(super) const MESSAGE: u32 = WM_APP + 0x259;
const PROPERTY: windows::core::PCWSTR = w!("Otter.LongCapture.Recording");
#[derive(Clone, Copy, Debug)]
pub(super) struct Wheel {
    pub point: POINT,
    pub time_ms: u64,
}
thread_local! {
    static WINDOW: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
    static ROOT: Cell<isize> = const { Cell::new(0) };
    static GENERATION: Cell<usize> = const { Cell::new(0) };
    static PENDING: Cell<Option<Wheel>> = const { Cell::new(None) };
    static QUEUED: Cell<bool> = const { Cell::new(false) };
}
pub(super) struct Hook {
    hwnd: HWND,
    token: usize,
    hook: HHOOK,
    _thread: PhantomData<Rc<()>>,
}
impl Hook {
    pub unsafe fn install(hwnd: HWND, root: isize) -> Result<Self> {
        if root == 0
            || !IsWindow(Some(hwnd)).as_bool()
            || GetWindowThreadProcessId(hwnd, None)
                != windows::Win32::System::Threading::GetCurrentThreadId()
            || WINDOW.with(|value| !value.get().is_invalid())
            || !GetPropW(hwnd, PROPERTY).is_invalid()
        {
            return Err(Error::new(
                E_UNEXPECTED,
                "Recording listener already active or invalid",
            ));
        }
        let token = GENERATION
            .with(|value| value.get().checked_add(1))
            .ok_or_else(|| Error::new(E_UNEXPECTED, "Recording generation exhausted"))?;
        let module = GetModuleHandleW(None)?;
        let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(callback), Some(module.into()), 0)?;
        if let Err(error) = SetPropW(hwnd, PROPERTY, Some(HANDLE(token as *mut _))) {
            let _ = UnhookWindowsHookEx(hook);
            return Err(error);
        }
        GENERATION.with(|value| value.set(token));
        ROOT.with(|value| value.set(root));
        PENDING.with(|value| value.set(None));
        QUEUED.with(|value| value.set(false));
        WINDOW.with(|value| value.set(hwnd));
        Ok(Self {
            hwnd,
            token,
            hook,
            _thread: PhantomData,
        })
    }
    pub unsafe fn take(&self, wp: WPARAM) -> Option<Wheel> {
        if wp.0 != self.token || !self.current() {
            return None;
        }
        QUEUED.with(|value| value.set(false));
        PENDING.with(Cell::take)
    }
    pub unsafe fn pending(&self) -> Option<Wheel> {
        if !self.current() {
            return None;
        }
        PENDING.with(Cell::take)
    }
    unsafe fn current(&self) -> bool {
        WINDOW.with(Cell::get) == self.hwnd
            && GENERATION.with(Cell::get) == self.token
            && GetPropW(self.hwnd, PROPERTY).0 as usize == self.token
    }
}
impl Drop for Hook {
    fn drop(&mut self) {
        // 先撤销回调可见状态；Unhook / RemoveProp 的重入不得再发布提示。
        WINDOW.with(|value| value.set(HWND::default()));
        ROOT.with(|value| value.set(0));
        PENDING.with(|value| value.set(None));
        QUEUED.with(|value| value.set(false));
        unsafe {
            let _ = UnhookWindowsHookEx(self.hook);
            if GetPropW(self.hwnd, PROPERTY).0 as usize == self.token {
                let _ = RemovePropW(self.hwnd, PROPERTY);
            }
        }
    }
}
#[cfg(test)]
pub(super) unsafe fn test_notify(point: POINT, root: isize) {
    notify(WM_MOUSEWHEEL, point, -120, root, GetTickCount64());
}
#[cfg(test)]
pub(super) fn test_inactive() -> bool {
    WINDOW.with(Cell::get).is_invalid() && PENDING.with(Cell::get).is_none()
}
unsafe extern "system" fn callback(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && lp.0 != 0 {
        let _ = std::panic::catch_unwind(|| {
            let data = &*(lp.0 as *const MSLLHOOKSTRUCT);
            if data.flags & 3 == 0 && matches!(wp.0 as u32, WM_MOUSEWHEEL | WM_MOUSEHWHEEL) {
                let root = GetAncestor(WindowFromPoint(data.pt), GA_ROOT).0 as isize;
                notify(
                    wp.0 as u32,
                    data.pt,
                    (data.mouseData >> 16) as u16 as i16 as i32,
                    root,
                    GetTickCount64(),
                );
            }
        });
    }
    CallNextHookEx(None, code, wp, lp)
}
unsafe fn notify(message: u32, point: POINT, delta: i32, root: isize, time_ms: u64) {
    let hwnd = WINDOW.with(Cell::get);
    if hwnd.is_invalid()
        || root == 0
        || ROOT.with(Cell::get) != root
        || delta == 0
        || !matches!(message, WM_MOUSEWHEEL | WM_MOUSEHWHEEL)
    {
        return;
    }
    PENDING.with(|value| value.set(Some(Wheel { point, time_ms })));
    if !QUEUED.with(|value| value.replace(true))
        && PostMessageW(
            Some(hwnd),
            MESSAGE,
            WPARAM(GENERATION.with(Cell::get)),
            LPARAM(0),
        )
        .is_err()
    {
        QUEUED.with(|value| value.set(false));
        PENDING.with(|value| value.set(None));
    }
}

#[cfg(test)]
#[path = "long_capture_recording_hook_tests.rs"]
mod tests;
