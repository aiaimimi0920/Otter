//! Toggle 读取实时编辑会话；沿用 change FIFO 与祖先 click 前缀屏障，不发送动作。
use super::*;
use crate::surface_scene::Kind;
use windows::Win32::UI::{Accessibility::*, Input::KeyboardAndMouse::IsWindowEnabled};

fn allowed(hwnd: HWND, state: &State, node: &Handle) -> Result<()> {
    if !current(state, node) {
        return Err(unavailable());
    }
    let entry = node.read().ok_or_else(unavailable)?;
    if entry.kind != Kind::Switch {
        return Err(Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)));
    }
    if entry.disabled
        || !state.local_edit.get()
        || !unsafe { IsWindowEnabled(hwnd) }.as_bool()
        || !unsafe { IsWindowVisible(hwnd) }.as_bool()
        || unsafe { IsIconic(hwnd) }.as_bool()
    {
        return Err(Error::from_hresult(HRESULT(UIA_E_ELEMENTNOTENABLED as i32)));
    }
    if state.pointer.borrow().automation_busy() {
        return Err(Error::from_hresult(HRESULT(UIA_E_INVALIDOPERATION as i32)));
    }
    Ok(())
}
pub(in crate::desktop::surface_window) fn value(hwnd: HWND, node: &Handle) -> Result<bool> {
    let entry = node.read().ok_or_else(unavailable)?;
    if entry.kind != Kind::Switch {
        return Err(Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)));
    }
    query(hwnd, node, |host| {
        host.accessibility_switch_value(&entry.id)
    })
    .map(|(_, value)| value)
}
pub(in crate::desktop::surface_window) fn change(
    hwnd: HWND,
    node: &Handle,
) -> Result<(bool, bool)> {
    let state = state(hwnd)?;
    allowed(hwnd, &state, node)?;
    let identity = state.pointer.borrow().identity();
    let id = node.read().ok_or_else(unavailable)?.id;
    let (_, (before, after, click)) = query(hwnd, node, |host| {
        if !host.accessibility_reveal_focus(&id)? {
            return Err(Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)));
        }
        allowed(hwnd, &state, node)?;
        if !state.pointer.borrow().same_identity(&identity) {
            return Err(unavailable());
        }
        let before = host.switch_value(&id).ok_or_else(unavailable)?;
        let has_click = state.pointer.borrow().enabled()
            && crate::surface_click_route::ClickRoute::new(host.presentation().scene(), Some(&id))
                .next()
                .is_some();
        let click = has_click.then(|| (host.capture_form_values(), host.input_fence()));
        if !host.accessibility_commit_toggle(&id)? {
            return Err(Error::from_hresult(HRESULT(UIA_E_INVALIDOPERATION as i32)));
        }
        Ok((
            before,
            host.switch_value(&id).ok_or_else(unavailable)?,
            click,
        ))
    })?;
    // 提交与入槽间无系统调用；祖先 click 使用 Toggle 之前的表单/屏障，保持既有次序。
    if let Some((values, fence)) = click {
        let queued = state
            .pointer
            .borrow_mut()
            .queue_automation(id, values, fence);
        debug_assert!(queued, "toggle reserved automation slot");
    }
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    Ok((before, after))
}
