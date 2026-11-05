//! HWND 私有恢复定时器：仅轮询既有退休 owner，完成前不绘制、不重建、不排队关窗。
use super::super::State;
use crate::desktop::composition_retirement::Status;
use windows::Win32::{Foundation::HWND, Graphics::Gdi::InvalidateRect, UI::WindowsAndMessaging::*};

pub(super) const TIMER: usize = 0x4F63;

pub(in crate::desktop) unsafe fn start(hwnd: HWND, state: &State) {
    if !state.lifetime.owns(hwnd) {
        return;
    }
    // 无 Canvas 借用跨 Win32 调度调用；失败保持原绘制错误，不启动另一候选。
    if SetTimer(Some(hwnd), TIMER, 16, None) == 0 {
        state.canvas.borrow_mut().suspend_render();
        eprintln!("native-render-recovery: timer=false document=retained");
        return;
    }
    poll(hwnd, state);
}

pub(super) unsafe fn poll(hwnd: HWND, state: &State) {
    if !state.lifetime.owns(hwnd) {
        return;
    }
    // 若重入活动 paint，延至下一 tick，禁止借用冲突或在外层帧期间更换 owner。
    if state.render_active.get() {
        return;
    }
    let status = state.canvas.borrow_mut().poll_render_recovery();
    if status == Some(Status::Pending) {
        return;
    }
    let _ = KillTimer(Some(hwnd), TIMER);
    match status {
        Some(Status::Complete) => {
            eprintln!("native-render-recovery: retirement=complete next-candidate=admitted document=retained");
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
        Some(status) => {
            eprintln!("native-render-recovery: retirement={status:?} document=retained")
        }
        None => {} // 已排队的旧 WM_TIMER 不再次创建、失效或清除原错误。
    }
}

pub(super) unsafe fn close(hwnd: HWND) {
    let _ = KillTimer(Some(hwnd), TIMER);
}
