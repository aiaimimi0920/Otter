//! Slider RangeValue 的 UI owner 桥；实时值来自 Mount，设置复用双边沿准入而非模拟键鼠。
use super::*;
use windows::Win32::UI::{Accessibility::*, Input::KeyboardAndMouse::IsWindowEnabled};
fn slider(node: &Handle) -> Result<crate::surface_accessibility::Entry> {
    let entry = node.read().ok_or_else(unavailable)?;
    if entry.kind != crate::surface_scene::Kind::Slider {
        return Err(Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)));
    }
    Ok(entry)
}
pub(in crate::desktop::surface_window) fn value(
    hwnd: HWND,
    node: &Handle,
) -> Result<(crate::surface_slider::Range, f64, bool)> {
    let entry = slider(node)?;
    query(hwnd, node, |host| {
        host.accessibility_slider_range(&entry.id)
    })
    .map(|(_, value)| value)
}
fn allowed(hwnd: HWND, state: &State, node: &Handle) -> Result<()> {
    if !current(state, node) {
        return Err(unavailable());
    }
    if slider(node)?.disabled
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
pub(in crate::desktop::surface_window) fn set(hwnd: HWND, node: &Handle, value: f64) -> Result<()> {
    if !value.is_finite() {
        return Err(Error::from_hresult(E_INVALIDARG));
    }
    let state = state(hwnd)?;
    allowed(hwnd, &state, node)?;
    let identity = state.pointer.borrow().identity();
    let id = slider(node)?.id;
    query(hwnd, node, |host| {
        if !host.accessibility_reveal_focus(&id)? {
            return Err(unavailable());
        }
        allowed(hwnd, &state, node)?;
        if !state.pointer.borrow().same_identity(&identity) {
            return Err(unavailable());
        }
        host.accessibility_set_slider_range(&id, value)
    })?;
    Ok(())
}
