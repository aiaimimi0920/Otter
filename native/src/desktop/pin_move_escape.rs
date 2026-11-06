//! 将拖动中的 Esc 原位转换为关闭消息，系统不再收到可触发位置回滚的按键。
use std::cell::Cell;
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        System::Threading::GetCurrentThreadId,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

thread_local! {
    static TARGET: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
}

pub(super) struct Guard(HHOOK);
impl Guard {
    pub(super) unsafe fn begin(hwnd: HWND) -> Result<Self> {
        let hook = SetWindowsHookExW(WH_GETMESSAGE, Some(filter), None, GetCurrentThreadId())?;
        TARGET.set(hwnd);
        Ok(Self(hook))
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        unsafe {
            if let Err(error) = UnhookWindowsHookEx(self.0) {
                eprintln!("native-pin-move-escape: {error}");
            }
        }
        TARGET.set(HWND::default());
    }
}

unsafe extern "system" fn filter(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wp.0 == PM_REMOVE.0 as usize && lp.0 != 0 {
        let message = &mut *(lp.0 as *mut MSG);
        if message.message == WM_KEYDOWN && message.wParam.0 == VK_ESCAPE.0 as usize {
            let hwnd = TARGET.get();
            if crate::desktop::host_lifetime::owns(hwnd) {
                let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const super::State);
                if super::can_close_on_escape(state, message.lParam) {
                    message.hwnd = hwnd;
                    message.message = WM_CLOSE;
                    message.wParam = WPARAM(0);
                    message.lParam = LPARAM(0);
                }
            }
        }
    }
    // 不在 hook 栈里销毁窗口；由原消息循环分派 WM_CLOSE，既不回滚也不先恢复宿主帧。
    CallNextHookEx(None, code, wp, lp)
}
