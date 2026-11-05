//! UI 线程内的 Weak 注册表。COM 不持 State 指针；查询期间升级 Rc 防止重入释放。
use super::*;
use crate::surface_accessibility::Handle;
use std::{collections::BTreeMap, rc::Weak};
use windows::Win32::UI::Accessibility::{UiaRect, UIA_E_ELEMENTNOTAVAILABLE};

thread_local! {
    static WINDOWS: RefCell<BTreeMap<usize, Weak<State>>> = const { RefCell::new(BTreeMap::new()) };
}
#[path = "surface_accessibility_focus.rs"]
pub(super) mod focus;
#[path = "surface_accessibility_invoke.rs"]
pub(super) mod invoke;
#[path = "surface_accessibility_edit.rs"]
pub(super) mod native_edit;
#[path = "surface_accessibility_range.rs"]
pub(super) mod range;
#[path = "surface_accessibility_toggle.rs"]
pub(super) mod toggle;
pub(super) fn register(hwnd: HWND, state: &Rc<State>) {
    state.accessibility_key.set(hwnd.0 as usize);
    WINDOWS.with(|windows| {
        windows
            .borrow_mut()
            .insert(hwnd.0 as usize, Rc::downgrade(state))
    });
}
pub(super) fn unregister(state: &State) {
    let key = state.accessibility_key.replace(0);
    if key != 0 {
        WINDOWS.with(|windows| {
            let mut windows = windows.borrow_mut();
            if windows
                .get(&key)
                .is_some_and(|entry| std::ptr::eq(entry.as_ptr(), state))
            {
                windows.remove(&key);
            }
        });
    }
}
fn unavailable() -> Error {
    Error::from_hresult(HRESULT(UIA_E_ELEMENTNOTAVAILABLE as i32))
}
pub(super) fn current(state: &State, handle: &Handle) -> bool {
    if state.closed.get() {
        return false;
    }
    let Some(entry) = handle.read() else {
        return false;
    };
    let expected = handle.runtime_id();
    expected.is_some()
        && state
            .accessibility
            .borrow()
            .as_ref()
            .and_then(|owner| owner.node(&entry.id))
            .and_then(|node| node.runtime_id())
            == expected
}
fn state(hwnd: HWND) -> Result<Rc<State>> {
    // 必须先释放注册表借用，再进入可能重入的 Host/Win32 调用。
    let state = WINDOWS
        .with(|windows| {
            windows
                .borrow()
                .get(&(hwnd.0 as usize))
                .and_then(Weak::upgrade)
        })
        .ok_or_else(unavailable)?;
    if state.closed.get() || state.accessibility.borrow().is_none() {
        return Err(unavailable());
    }
    Ok(state)
}
fn query<T>(
    hwnd: HWND,
    handle: &Handle,
    operation: impl FnOnce(&mut Host) -> Result<T>,
) -> Result<(Rc<State>, T)> {
    let state = state(hwnd)?;
    if !current(&state, handle) {
        return Err(unavailable());
    }
    let mut result = None;
    let ran = state.run(|host| {
        result = Some(operation(host)?);
        Ok(())
    })?;
    if !ran || !current(&state, handle) {
        return Err(unavailable());
    }
    Ok((state, result.ok_or_else(unavailable)?))
}
pub(super) fn bounds(hwnd: HWND, handle: &Handle) -> Result<(UiaRect, bool)> {
    let id = handle.read().ok_or_else(unavailable)?.id;
    query(hwnd, handle, |host| host.accessibility_bounds(&id)).map(|(_, value)| value)
}
pub(super) fn offscreen(hwnd: HWND, handle: &Handle) -> Result<bool> {
    let id = handle.read().ok_or_else(unavailable)?.id;
    query(hwnd, handle, |host| {
        if unsafe { !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() } {
            return Ok(true);
        }
        host.accessibility_bounds(&id)
            .map(|(_, offscreen)| offscreen)
    })
    .map(|(_, value)| value)
}
pub(super) enum Hit {
    Outside,
    Root,
    Node(Handle),
}
pub(super) fn hit(hwnd: HWND, scene: &Handle, point: [f64; 2]) -> Result<Hit> {
    if point.iter().any(|value| !value.is_finite()) {
        return Err(Error::from_hresult(E_INVALIDARG));
    }
    let (state, id) = query(hwnd, scene, |host| {
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect)? };
        if unsafe { !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() }
            || point[0] < rect.left as f64
            || point[0] >= rect.right as f64
            || point[1] < rect.top as f64
            || point[1] >= rect.bottom as f64
        {
            return Ok(None);
        }
        Ok(Some(host.accessibility_hit(point)?))
    })?;
    let Some(id) = id else {
        return Ok(Hit::Outside);
    };
    let Some(id) = id else {
        return Ok(Hit::Root);
    };
    let node = state
        .accessibility
        .borrow()
        .as_ref()
        .and_then(|owner| owner.node(&id))
        .ok_or_else(unavailable)?;
    if matches!(
        node.read().ok_or_else(unavailable)?.kind,
        crate::surface_scene::Kind::Input
            | crate::surface_scene::Kind::Textarea
            | crate::surface_scene::Kind::Number
            | crate::surface_scene::Kind::Slider
    ) && native_edit::hwnd(hwnd, &node)?.is_none()
    {
        return Ok(Hit::Outside);
    }
    Ok(Hit::Node(node))
}

#[cfg(test)]
#[path = "surface_accessibility_bridge_tests.rs"]
mod tests;
