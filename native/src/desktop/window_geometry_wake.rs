//! sent 几何消息不保证 GetMessage 返回；每 HWND 合并一条 queued 通知，磁盘/模型仍归应用结算。
use std::cell::Cell;
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

const MESSAGE: u32 = WM_APP + 117;
pub(super) struct Wake {
    active: Cell<bool>,
    pending: Cell<bool>,
}
impl Default for Wake {
    fn default() -> Self {
        Self {
            active: Cell::new(true),
            pending: Cell::new(false),
        }
    }
}
impl Wake {
    fn request(&self, flags: SET_WINDOW_POS_FLAGS, send: impl FnOnce() -> bool) {
        if self.active.get() && !self.pending.get() && !flags.contains(SWP_NOMOVE | SWP_NOSIZE) {
            // PostMessage 不同步重入窗口过程；投递失败不锁住 pending，已有队列消息可驱动结算。
            self.pending.set(send());
        }
    }
    fn close(&self) {
        self.active.set(false);
        self.pending.set(false);
    }
}
pub(super) unsafe fn dispatch(hwnd: HWND, wake: &Wake, msg: u32, lp: LPARAM) -> Option<LRESULT> {
    match msg {
        WM_WINDOWPOSCHANGED if lp.0 != 0 => {
            wake.request((*(lp.0 as *const WINDOWPOS)).flags, || {
                PostMessageW(Some(hwnd), MESSAGE, WPARAM(0), LPARAM(0)).is_ok()
            });
            // 必须继续 DefWindowProc 的 WM_MOVE/WM_SIZE 默认派发，不能吞掉 resize/DPI/布局。
            None
        }
        MESSAGE => {
            wake.pending.set(false); // 先 ack；本轮结算内的后续几何变化能再投递一次。
            Some(LRESULT(0))
        }
        WM_DESTROY | WM_NCDESTROY => {
            wake.close(); // State 仍存活也不能向已销毁、可能复用的 HWND 投递。
            None
        }
        _ => None,
    }
}

#[cfg(test)]
#[path = "window_geometry_wake_tests.rs"]
mod tests;
