//! 轨道重复接线；每次 tick 重新核验 capture、本地权限和当前轨道命中。
use super::*;
use std::time::Instant;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, SetCapture};
pub(super) fn start(
    hwnd: HWND,
    state: &State,
    bar: &crate::surface_scroll::Bar,
    logical: [f32; 2],
    pixels: [i32; 2],
) -> Result<()> {
    let direction = bar.page_direction(logical);
    if direction == 0 {
        return Ok(());
    }
    let repeat = scroll_repeat::Repeat::new(hwnd, bar.id.clone(), bar.axis, direction, pixels)?;
    let id = repeat.id();
    *state.scroll_repeat.borrow_mut() = Some(repeat);
    unsafe {
        let _ = SetCapture(hwnd);
        // 同步重入可能取消或替换手势；旧 start 不能驱动或清掉新 owner。
        let current = state.scroll_repeat.borrow().as_ref().map(|r| r.id());
        if current != Some(id) {
            if current.is_none() {
                pointer::cancel(hwnd, state);
            }
            return Ok(());
        }
        if GetCapture() != hwnd || state.closed.get() || !state.local_edit.get() {
            pointer::cancel(hwnd, state);
            return Ok(());
        }
    }
    state.run(|host| host.page_scrollbar(&bar.id, bar.axis, direction, pixels))?;
    Ok(())
}
pub(super) fn timer(hwnd: HWND, state: &State, id: usize, now: Instant) -> bool {
    if pointer::text_drag_timer(hwnd, state, id, now) {
        return true;
    }
    let matches = state
        .scroll_repeat
        .borrow()
        .as_ref()
        .is_some_and(|r| r.id() == id);
    if !matches {
        return false;
    }
    if state.closed.get() || !state.local_edit.get() || unsafe { GetCapture() } != hwnd {
        pointer::cancel(hwnd, state);
        return true;
    }
    let next = {
        let mut repeat = state.scroll_repeat.borrow_mut();
        repeat.as_mut().map(|r| {
            r.ready(id, now)
                .map(|ready| ready.then(|| (r.node.clone(), r.axis, r.direction, r.point)))
        })
    };
    let result = match next {
        Some(Ok(Some((node, axis, direction, point)))) => {
            state.run(|host| host.page_scrollbar(&node, axis, direction, point))
        }
        Some(Err(error)) => Err(error),
        _ => return true,
    };
    if let Err(error) = result {
        *state.error.borrow_mut() = Some(error);
        pointer::invalidate(hwnd, state);
    }
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    true
}
