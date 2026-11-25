//! UIA Invoke submits only bounded candidates, retaining form snapshots/FIFO fences without network calls or focus stealing.
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
    // Do not take over an already started mouse gesture, whose mouse-up would overwrite accepted automation requests.
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
    // EDIT synchronization can reenter; revocation followed by reauthorization must not revive a prior activation edge.
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
