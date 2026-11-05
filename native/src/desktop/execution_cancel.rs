//! 原生取消菜单只捕获请求身份并唤醒应用，不在 Win32 回调内等待后台清理。
use super::State;
use crate::{graph_requests, workspace_id::UnitId};
use windows::{
    core::{w, Result},
    Win32::{Foundation::*, UI::WindowsAndMessaging::*},
};
pub(super) const COMMAND: usize = 0x1080;
pub(super) struct Request {
    pub preparation: Option<UnitId>,
    pub execution: Option<graph_requests::Request>,
}
fn snapshot(state: &State) -> Request {
    Request {
        preparation: state
            .pending_preparation
            .borrow()
            .as_ref()
            .map(|pending| pending.ticket.clone()),
        execution: state
            .pending_execution
            .borrow()
            .as_ref()
            .map(|execution| execution.request.clone())
            .or_else(|| state.execution_results.borrow().active().cloned()),
    }
}
pub(super) unsafe fn append(menu: HMENU, state: &State) -> Result<()> {
    let request = snapshot(state);
    let enabled = request.preparation.is_some()
        || request.execution.is_some()
        || state.pending_start.borrow().is_some();
    AppendMenuW(
        menu,
        MF_STRING | if enabled { MF_ENABLED } else { MF_GRAYED },
        COMMAND,
        w!("停止节点执行"),
    )
}
/// 显式停止包含排队请求；参数编辑退休旧执行时仍使用 request，不能删除新请求。
pub(super) unsafe fn stop(hwnd: HWND, state: &State) -> Result<()> {
    if !IsWindow(Some(hwnd)).as_bool() {
        return Err(windows::core::Error::from_win32());
    }
    let queued = state.pending_start.borrow_mut().take();
    if let Err(error) = request(hwnd, state) {
        *state.pending_start.borrow_mut() = queued;
        return Err(error);
    }
    Ok(())
}
pub(super) unsafe fn request(hwnd: HWND, state: &State) -> Result<()> {
    let request = snapshot(state);
    if request.preparation.is_none() && request.execution.is_none() {
        return Ok(());
    }
    let previous = state.execution_cancel.replace(Some(request));
    if let Err(error) = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0)) {
        state.execution_cancel.replace(previous);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_wakeup_does_not_leave_cancellation() {
        let state = State::new(None, false).unwrap();
        let request = graph_requests::Request {
            id: UnitId::parse("request").unwrap(),
            generation: 1,
        };
        state.execution_results.borrow_mut().begin(request.clone());
        assert!(unsafe { super::request(HWND(-2isize as *mut _), &state) }.is_err());
        assert!(state.execution_cancel.borrow().is_none());
        assert_eq!(state.execution_results.borrow().active(), Some(&request));
    }
}
