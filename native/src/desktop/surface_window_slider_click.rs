//! Slider 子 HWND 鼠标完成边沿；只复用父 click 单槽与既有 FIFO fence，不发送网络。
use super::*;
use crate::desktop::surface_slider_control::{CLICK_BEGIN, CLICK_CANCEL, CLICK_END};
pub(super) struct Press {
    child: HWND,
    node: String,
    scene: Rc<()>,
}
unsafe fn point_in_parent(
    parent: HWND,
    child: HWND,
    packed: WPARAM,
    require_inside: bool,
) -> Option<[i32; 2]> {
    let mut point = POINT {
        x: (packed.0 as u16 as i16) as i32,
        y: ((packed.0 >> 16) as u16 as i16) as i32,
    };
    let mut rect = RECT::default();
    if GetClientRect(child, &mut rect).is_err()
        || (require_inside
            && (point.x < 0 || point.y < 0 || point.x >= rect.right || point.y >= rect.bottom))
    {
        return None;
    }
    // 本控件坐标转宿主坐标；MapWindowPoints 返回 0 也可能是合法零偏移。
    MapWindowPoints(Some(child), Some(parent), std::slice::from_mut(&mut point));
    Some([point.x, point.y])
}
pub(in crate::desktop::surface_window) unsafe fn message(
    parent: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    if ![CLICK_BEGIN, CLICK_CANCEL, CLICK_END].contains(&message) {
        return false;
    }
    let child = HWND(lp.0 as *mut _);
    if message == CLICK_CANCEL {
        let mut pointer = state.pointer.borrow_mut();
        if pointer
            .native_press
            .as_ref()
            .is_some_and(|press| press.child == child)
        {
            pointer.native_press = None;
        }
        return true;
    }
    let previous = if message == CLICK_END {
        let mut pointer = state.pointer.borrow_mut();
        if !pointer
            .native_press
            .as_ref()
            .is_some_and(|press| press.child == child)
        {
            return true;
        }
        pointer.native_press.take()
    } else {
        None
    };
    if state.closed.get()
        || !state.local_edit.get()
        || !state.pointer.borrow().enabled
        || state.pointer.borrow().automation_busy()
    {
        return true;
    }
    if !IsWindowEnabled(child).as_bool() || GetParent(child).ok() != Some(parent) {
        return true;
    }
    let Some(point) = point_in_parent(parent, child, wp, message == CLICK_BEGIN) else {
        return true;
    };
    let scene = state.pointer.borrow().identity();
    if previous
        .as_ref()
        .is_some_and(|press| !Rc::ptr_eq(&press.scene, &scene))
    {
        return true;
    }
    let mut target = None;
    let result = state.run(|host| {
        let Some(id) = host.accessibility_edit_id(child) else {
            return Ok(());
        };
        if host.slider_readonly(&id) != Some(false)
            || !host.focusable(&id)?
            || (message == CLICK_BEGIN && host.hit_test(point)? != Some(id.as_str()))
        {
            return Ok(());
        }
        if crate::surface_click_route::ClickRoute::new(host.presentation().scene(), Some(&id))
            .next()
            .is_none()
        {
            return Ok(());
        }
        target = Some(id);
        Ok(())
    });
    match result {
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            return true;
        }
        Ok(false) => return true,
        Ok(true) => {}
    }
    if state.closed.get() || !state.pointer.borrow().same_identity(&scene) {
        return true;
    }
    let Some(node) = target else {
        return true;
    };
    if message == CLICK_BEGIN {
        if GetCapture() == child {
            state.pointer.borrow_mut().native_press = Some(Press { child, node, scene });
        }
        return true;
    }
    if previous.is_none_or(|press| press.node != node) {
        return true;
    }
    // Slider change 已经入队，祖先 click 必须等待它，而非采用 Switch 的前置 fence。
    let Some((values, fence)) = form_snapshot(state) else {
        return true;
    };
    let mut pointer = state.pointer.borrow_mut();
    if pointer.enabled && !pointer.automation_busy() && pointer.same_identity(&scene) {
        pointer.pending = Some(Click {
            client_pixels: point,
            node_id: node,
            modifiers: modifiers(),
            scene,
            source: Activation::CapturedPointer,
            values,
            fence,
            captured_at: std::time::Instant::now(),
        });
    }
    true
}
