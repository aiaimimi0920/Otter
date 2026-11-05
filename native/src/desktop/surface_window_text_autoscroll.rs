//! 只有边缘拖选时持有 HWND timer；不追赶积压 tick，撤销即释放，迟到身份不复活。
use super::*;
use std::time::{Duration, Instant};
const INTERVAL: u32 = 50;
pub(in crate::desktop::surface_window) struct Timer {
    hwnd: HWND,
    id: usize,
    point: [i32; 2],
    due: Instant,
}
impl Timer {
    fn new(hwnd: HWND, point: [i32; 2]) -> Result<Self> {
        let id = scroll_repeat::allocate_id()?;
        let due = Instant::now() + Duration::from_millis(INTERVAL.into());
        if unsafe { SetTimer(Some(hwnd), id, INTERVAL, None) } == 0 {
            return Err(Error::from_win32());
        }
        Ok(Self {
            hwnd,
            id,
            point,
            due,
        })
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        unsafe {
            let _ = KillTimer(Some(self.hwnd), self.id);
        }
    }
}
pub(super) fn update(hwnd: HWND, state: &State, point: [i32; 2]) {
    update_using(hwnd, state, point, || {
        apply(hwnd, state, |host| host.text_drag_scroll(point, false))
    });
}
fn update_using(hwnd: HWND, state: &State, point: [i32; 2], pending: impl FnOnce() -> bool) {
    let Some(owner) = state.pointer.borrow().selection_owner.clone() else {
        return;
    };
    let pending = pending();
    // 查询会同步重入撤权/取消/新手势；旧结果不得启动或清掉新 owner 的 timer。
    if !state
        .pointer
        .borrow()
        .selection_owner
        .as_ref()
        .is_some_and(|current| Rc::ptr_eq(current, &owner))
    {
        return;
    }
    if state.closed.get()
        || !state.local_edit.get()
        || unsafe { GetCapture() != hwnd || GetFocus() != hwnd }
    {
        super::super::cancel(hwnd, state);
        return;
    }
    if !pending {
        state.pointer.borrow_mut().selection_scroll = None;
        return;
    }
    if let Some(timer) = state.pointer.borrow_mut().selection_scroll.as_mut() {
        timer.point = point;
        return;
    }
    match Timer::new(hwnd, point) {
        Ok(timer) => state.pointer.borrow_mut().selection_scroll = Some(timer),
        Err(error) => {
            *state.error.borrow_mut() = Some(error);
            super::super::cancel(hwnd, state);
        }
    }
}
pub(in crate::desktop::surface_window) fn timer(
    hwnd: HWND,
    state: &State,
    id: usize,
    now: Instant,
) -> bool {
    let point = {
        let mut pointer = state.pointer.borrow_mut();
        let Some(timer) = pointer
            .selection_scroll
            .as_mut()
            .filter(|timer| timer.id == id)
        else {
            return false;
        };
        if now < timer.due {
            return true;
        }
        timer.due = now + Duration::from_millis(INTERVAL.into());
        timer.point
    };
    if state.closed.get()
        || !state.local_edit.get()
        || unsafe { GetCapture() != hwnd || GetFocus() != hwnd }
    {
        super::super::cancel(hwnd, state);
        return true;
    }
    let changed = apply(hwnd, state, |host| host.text_drag_scroll(point, true));
    if !changed {
        let mut pointer = state.pointer.borrow_mut();
        if pointer
            .selection_scroll
            .as_ref()
            .is_some_and(|timer| timer.id == id)
        {
            pointer.selection_scroll = None;
        }
    }
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    true
}
#[cfg(test)]
#[path = "surface_window_text_autoscroll_tests.rs"]
mod tests;
