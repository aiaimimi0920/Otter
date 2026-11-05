//! UIA 焦点桥接；复用键盘 owner/显露/候选撤销，绝不生成 click/change。
use super::*;
use crate::surface_scene::Kind;
use windows::Win32::UI::{
    Accessibility::*,
    Input::KeyboardAndMouse::{GetFocus, IsWindowEnabled, SetFocus},
};

pub(in crate::desktop::surface_window) enum Focus {
    None,
    Root,
    Node(Handle),
}
pub(in crate::desktop::surface_window) struct Properties {
    pub enabled: bool,
    pub focusable: bool,
    pub focused: bool,
}
fn validate(state: &State, scene: Option<&Handle>) -> Result<()> {
    let valid = if let Some(scene) = scene {
        current(state, scene)
    } else {
        !state.closed.get()
            && state
                .accessibility
                .borrow()
                .as_ref()
                .is_some_and(|owner| owner.root().is_none())
    };
    if valid {
        Ok(())
    } else {
        Err(unavailable())
    }
}
fn visible(hwnd: HWND) -> bool {
    unsafe { IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() }
}
pub(super) fn allowed(hwnd: HWND, state: &State) -> Result<()> {
    if !state.local_edit.get() || !visible(hwnd) || !unsafe { IsWindowEnabled(hwnd) }.as_bool() {
        Err(Error::from_hresult(HRESULT(UIA_E_ELEMENTNOTENABLED as i32)))
    } else {
        Ok(())
    }
}
fn control(handle: &Handle) -> bool {
    handle
        .read()
        .is_some_and(|entry| matches!(entry.kind, Kind::Button | Kind::Switch))
}
pub(in crate::desktop::surface_window) fn get(hwnd: HWND, scene: Option<&Handle>) -> Result<Focus> {
    let state = state(hwnd)?;
    validate(&state, scene)?;
    if !visible(hwnd) {
        return Ok(Focus::None);
    }
    let focused = unsafe { GetFocus() };
    if focused != hwnd {
        // 原生 EDIT 已重定位到本 fragment；返回承载节点而不是误报 fragment 无焦点。
        return super::native_edit::node(hwnd, focused)
            .map(|node| node.map_or(Focus::None, Focus::Node));
    }
    let id = state.keyboard.borrow().focused().map(str::to_owned);
    let Some(id) = id else {
        return Ok(Focus::Root);
    };
    let handle = state
        .accessibility
        .borrow()
        .as_ref()
        .and_then(|owner| owner.node(&id))
        .ok_or_else(unavailable)?;
    if !control(&handle) || !state.local_edit.get() {
        return Ok(Focus::Root);
    }
    let (_, eligible) = query(hwnd, &handle, |host| host.accessibility_can_focus(&id))?;
    validate(&state, scene)?;
    if unsafe { GetFocus() } != hwnd || state.keyboard.borrow().focused() != Some(id.as_str()) {
        return Err(unavailable());
    }
    Ok(if eligible && state.local_edit.get() && visible(hwnd) {
        Focus::Node(handle)
    } else {
        Focus::Root
    })
}
pub(in crate::desktop::surface_window) fn properties(
    hwnd: HWND,
    scene: Option<&Handle>,
    node: Option<&Handle>,
) -> Result<Properties> {
    let state = state(hwnd)?;
    validate(&state, scene)?;
    if let Some(node) = node {
        if !current(&state, node) {
            return Err(unavailable());
        }
        let entry = node.read().ok_or_else(unavailable)?;
        let interactive = matches!(entry.kind, Kind::Button | Kind::Switch | Kind::Slider);
        let candidate =
            interactive && query(hwnd, node, |host| host.accessibility_can_focus(&entry.id))?.1;
        // Host 投影可能同步重入；完成查询后再读取权限和真实焦点。
        let focus = get(hwnd, scene)?;
        let window_enabled = unsafe { IsWindowEnabled(hwnd) }.as_bool();
        let enabled =
            window_enabled && (!interactive || (!entry.disabled && state.local_edit.get()));
        let focusable = enabled && candidate && visible(hwnd);
        let focused =
            matches!(focus, Focus::Node(ref current) if current.runtime_id() == node.runtime_id());
        validate(&state, scene)?;
        if !current(&state, node) {
            return Err(unavailable());
        }
        Ok(Properties {
            enabled,
            focusable,
            focused,
        })
    } else {
        let focus = get(hwnd, scene)?;
        let window_enabled = unsafe { IsWindowEnabled(hwnd) }.as_bool();
        Ok(Properties {
            enabled: window_enabled,
            focusable: window_enabled && state.local_edit.get() && visible(hwnd),
            focused: matches!(focus, Focus::Root),
        })
    }
}
pub(in crate::desktop::surface_window) fn set(
    hwnd: HWND,
    scene: Option<&Handle>,
    node: Option<&Handle>,
) -> Result<()> {
    let state = state(hwnd)?;
    validate(&state, scene)?;
    allowed(hwnd, &state)?;
    if state.host.borrow().is_none() {
        return Err(unavailable());
    }
    let id = if let Some(node) = node {
        if !current(&state, node) {
            return Err(unavailable());
        }
        let entry = node.read().ok_or_else(unavailable)?;
        if !control(node) {
            return Err(Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)));
        }
        if entry.disabled {
            return Err(Error::from_hresult(HRESULT(UIA_E_ELEMENTNOTENABLED as i32)));
        }
        if !query(hwnd, node, |host| {
            host.accessibility_reveal_focus(&entry.id)
        })?
        .1
        {
            return Err(Error::from_hresult(HRESULT(UIA_E_NOTSUPPORTED as i32)));
        }
        Some(entry.id)
    } else {
        None
    };
    validate(&state, scene)?;
    allowed(hwnd, &state)?;
    pointer::invalidate(hwnd, &state);
    // 取消 capture、SetFocus 都可能同步重入，期间不持 RefCell/Host 借用。
    validate(&state, scene)?;
    allowed(hwnd, &state)?;
    unsafe {
        let _ = SetFocus(Some(hwnd));
    }
    validate(&state, scene)?;
    allowed(hwnd, &state)?;
    if unsafe { GetFocus() } != hwnd {
        return Err(Error::from_hresult(E_FAIL));
    }
    if let (Some(node), Some(id)) = (node, id.as_deref()) {
        if !query(hwnd, node, |host| host.focusable(id))?.1 {
            return Err(unavailable());
        }
    }
    validate(&state, scene)?;
    allowed(hwnd, &state)?;
    if unsafe { GetFocus() } != hwnd {
        return Err(unavailable());
    }
    keyboard::set_focus(&state, id);
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    Ok(())
}
