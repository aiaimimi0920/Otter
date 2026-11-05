//! Switch 激活接缝；本地 editing 门禁与事件 submitting 门禁分离，不在回调发送动作。
use super::*;
pub(super) fn has_click(state: &State, id: &str) -> bool {
    state.host.borrow().as_ref().is_some_and(|host| {
        crate::surface_click_route::ClickRoute::new(host.presentation().scene(), Some(id))
            .next()
            .is_some()
    })
}
pub(super) fn is_switch(state: &State, id: &str) -> bool {
    state
        .host
        .borrow()
        .as_ref()
        .is_some_and(|host| host.switch_value(id).is_some())
}
pub(super) fn activate(hwnd: HWND, state: &State, id: &str) -> Option<bool> {
    if !is_switch(state, id) {
        return None;
    }
    if state.closed.get() || !state.local_edit.get() {
        return Some(false);
    }
    let mut changed = false;
    if let Err(error) = state.run(|host| {
        changed = host.toggle_switch(id)?;
        Ok(())
    }) {
        *state.error.borrow_mut() = Some(error);
    }
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    Some(changed)
}
