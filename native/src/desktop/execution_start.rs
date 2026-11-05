//! 用户启动命令只捕获配置身份并唤醒应用；不在 Win32 回调中编码图片或连接 Loom。
use super::{configuration_stamp::Stamp, State};
use windows::{
    core::{w, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x1083;
#[cfg(test)]
#[path = "execution_start_restore_tests.rs"]
mod restore_tests;
pub(super) struct Request {
    pub stamp: Stamp,
    pub trigger: crate::graph_execution_config::Trigger,
}
/// 参数事务完成后合并为最新配置；旧执行先由取消队列退休，不能在编辑回调启动 worker。
pub(super) fn parameter_changed(state: &State) {
    automatic_changed(
        state,
        crate::graph_execution_config::Trigger::Parameter { final_change: true },
    );
}
pub(super) fn upstream_changed(state: &State) {
    automatic_changed(state, crate::graph_execution_config::Trigger::Upstream);
}
fn automatic_changed(state: &State, automatic: crate::graph_execution_config::Trigger) {
    if state.group_locked.get() || state.selected_art.borrow().is_none() {
        return;
    }
    // 真正的参数/上游变动解除恢复预览保持，即使自动执行开关稍后拒绝后端请求。
    state
        .execution_results
        .borrow_mut()
        .invalidate_restored_preview();
    let mut pending = state.pending_start.borrow_mut();
    let trigger = if pending
        .as_ref()
        .is_some_and(|request| request.trigger == crate::graph_execution_config::Trigger::Manual)
    {
        crate::graph_execution_config::Trigger::Manual
    } else {
        automatic
    };
    *pending = Some(Request {
        stamp: Stamp::capture(state),
        trigger,
    });
}
pub(super) fn available(state: &State) -> bool {
    ready(state) && state.pending_start.borrow().is_none()
}
/// 调度器检查不包含待启动槽本身，避免排队请求把自己永久判为繁忙。
pub(super) fn ready(state: &State) -> bool {
    state.selected_art.borrow().is_some()
        && !state.group_locked.get()
        && state.pending_preparation.borrow().is_none()
        && state.pending_execution.borrow().is_none()
        && state.execution_results.borrow().active().is_none()
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    AppendMenuW(
        menu,
        MF_STRING
            | if available(state) {
                MF_ENABLED
            } else {
                MF_GRAYED
            },
        COMMAND,
        w!("运行所选能力"),
    )
}
pub(super) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(windows::core::Error::from_win32());
    }
    if !available(state) {
        return Ok(());
    }
    *state.pending_start.borrow_mut() = Some(Request {
        stamp: Stamp::capture(state),
        trigger: crate::graph_execution_config::Trigger::Manual,
    });
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        state.pending_start.borrow_mut().take();
        return Err(error);
    }
    state
        .execution_results
        .borrow_mut()
        .invalidate_restored_preview();
    Ok(())
}
/// modeless 草稿提交后的同次显式运行：只升级当前参数触发，不丢弃/覆盖别的启动请求。
pub(super) unsafe fn request_after_parameter(hwnd: HWND, state: &State) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() || !ready(state) {
        return Ok(());
    }
    let promote = state
        .pending_start
        .borrow()
        .as_ref()
        .is_some_and(|request| {
            request.stamp.matches(state)
                && matches!(
                    request.trigger,
                    crate::graph_execution_config::Trigger::Parameter { .. }
                )
        });
    if !promote {
        return request(hwnd, state);
    }
    PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0))?;
    // PostMessage 不同步重入，入队成功后再升级；失败保留原自动触发。
    state.pending_start.borrow_mut().as_mut().unwrap().trigger =
        crate::graph_execution_config::Trigger::Manual;
    Ok(())
}
