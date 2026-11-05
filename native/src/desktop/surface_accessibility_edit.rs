//! 只映射当前挂载 EDIT/COMBOBOX；不缓存裸 HWND、不遍历外部窗口、不复制系统模式。
use super::*;
pub(in crate::desktop::surface_window) fn focus(parent: HWND, node: &Handle) -> Result<()> {
    let state = state(parent)?;
    super::focus::allowed(parent, &state)?;
    let entry = node.read().ok_or_else(unavailable)?;
    if entry.disabled {
        return Err(Error::from_hresult(HRESULT(
            windows::Win32::UI::Accessibility::UIA_E_ELEMENTNOTENABLED as i32,
        )));
    }
    if !query(parent, node, |host| {
        host.accessibility_reveal_focus(&entry.id)
    })?
    .1
    {
        return Err(unavailable());
    }
    super::focus::allowed(parent, &state)?;
    let expected = hwnd(parent, node)?.ok_or_else(unavailable)?;
    if !query(parent, node, |host| host.focus_edit(&entry.id))?.1 {
        return Err(unavailable());
    }
    super::focus::allowed(parent, &state)?;
    if hwnd(parent, node)? != Some(expected)
        || unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetFocus() } != expected
    {
        return Err(unavailable());
    }
    Ok(())
}
pub(in crate::desktop::surface_window) fn hwnd(
    parent: HWND,
    node: &Handle,
) -> Result<Option<HWND>> {
    let id = node.read().ok_or_else(unavailable)?.id;
    query(parent, node, |host| Ok(host.accessibility_edit_hwnd(&id))).map(|(_, hwnd)| hwnd)
}
pub(in crate::desktop::surface_window) fn node(
    parent: HWND,
    child: HWND,
) -> Result<Option<Handle>> {
    let state = state(parent)?;
    let host = state.host.borrow();
    let host = host.as_ref().ok_or_else(unavailable)?;
    let Some(id) = host.accessibility_edit_id(child) else {
        return Ok(None);
    };
    let node = state
        .accessibility
        .borrow()
        .as_ref()
        .and_then(|owner| owner.node(&id));
    Ok(node)
}
