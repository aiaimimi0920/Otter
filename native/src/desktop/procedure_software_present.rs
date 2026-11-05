//! 软件 HWND 样式和首帧提交为同一事务；跨重入只持不可变 DIB，不借用 Canvas。
use super::super::{canvas::Pending, software_surface::window_style::Transition, State};
use windows::{core::Result, Win32::Foundation::HWND};

pub(super) unsafe fn submit(hwnd: HWND, state: &State, pending: Pending) -> Result<bool> {
    let owns = || state.lifetime.owns(hwnd);
    if !owns() {
        return Ok(false);
    }
    let transition = Transition::begin_guarded(hwnd, pending.pinned, &owns)?;
    if !state.canvas.borrow().accepts_pending(&pending) {
        state.render_deferred.set(true);
        return Ok(false); // 回滚样式，不提交重入前的旧文档帧。
    }
    let presented = pending.present(hwnd)?;
    if !owns() {
        return Ok(false);
    }
    if !state.canvas.borrow().accepts_presented(&presented) {
        state.render_deferred.set(true);
        return Ok(false);
    }
    transition.commit()?;
    Ok(state.canvas.borrow_mut().confirm_submission(presented))
}
