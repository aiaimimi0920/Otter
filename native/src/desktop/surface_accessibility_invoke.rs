//! UIA Invoke 只提交有界候选；沿用表单快照/FIFO fence，不调用网络或抢占焦点。
use super::*;
use crate::surface_scene::Kind;
use windows::Win32::UI::{Accessibility::*, Input::KeyboardAndMouse::IsWindowEnabled};

fn allowed(hwnd: HWND, state: &State, node: &Handle) -> Result<()> {
    if !current(state, node) {
        return Err(unavailable());
    }
    let entry = node.read().ok_or_else(unavailable)?;
    if entry.kind != Kind::Button {
        return Err(Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)));
    }
    if entry.disabled
        || !state.local_edit.get()
        || !state.pointer.borrow().enabled()
        || !unsafe { IsWindowEnabled(hwnd) }.as_bool()
        || !unsafe { IsWindowVisible(hwnd) }.as_bool()
        || unsafe { IsIconic(hwnd) }.as_bool()
    {
        return Err(Error::from_hresult(HRESULT(UIA_E_ELEMENTNOTENABLED as i32)));
    }
    // 不接管已开始的鼠标手势，否则其 mouse-up 会覆盖已接受的自动化请求。
    if state.pointer.borrow().automation_busy() {
        return Err(Error::from_hresult(HRESULT(UIA_E_INVALIDOPERATION as i32)));
    }
    Ok(())
}
pub(in crate::desktop::surface_window) fn enqueue(hwnd: HWND, node: &Handle) -> Result<()> {
    let state = state(hwnd)?;
    allowed(hwnd, &state, node)?;
    let identity = state.pointer.borrow().identity();
    let id = node.read().ok_or_else(unavailable)?.id;
    let (_, captured) = query(hwnd, node, |host| {
        if !host.accessibility_reveal_focus(&id)? {
            return Err(Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)));
        }
        Ok((host.capture_form_values(), host.input_fence()))
    })?;
    // EDIT 同步可重入；撤权后重新授权也不能复活此前的激活边沿。
    allowed(hwnd, &state, node)?;
    let mut pointer = state.pointer.borrow_mut();
    if !pointer.same_identity(&identity) {
        return Err(unavailable());
    }
    if !pointer.queue_automation(id, captured.0, captured.1) {
        return Err(Error::from_hresult(HRESULT(UIA_E_INVALIDOPERATION as i32)));
    }
    Ok(())
}
