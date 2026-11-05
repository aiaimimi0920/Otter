//! Live 本地拖动独占捕获；以初始屏幕坐标计算，不在移动后的客户区累计误差。
use super::*;
use crate::pin_geometry::PinRect;

#[derive(Clone, Copy)]
pub(in crate::desktop::composition::live::window) struct Drag {
    rect: PinRect,
    pointer: (i32, i32),
    resize: bool,
    owner: u64,
}
pub(in crate::desktop::composition::live::window) unsafe fn cancel(hwnd: HWND, state: &State) {
    state.drag_starting.set(false);
    // 先退休状态，再释放捕获；ReleaseCapture 会同步重入 WM_CAPTURECHANGED。
    if state.drag.take().is_some() && GetCapture() == hwnd {
        let _ = ReleaseCapture();
    }
}
unsafe fn screen_point(hwnd: HWND, lp: LPARAM) -> Option<(i32, i32)> {
    let mut point = POINT {
        x: lp.0 as u16 as i16 as i32,
        y: (lp.0 >> 16) as u16 as i16 as i32,
    };
    ClientToScreen(hwnd, &mut point)
        .as_bool()
        .then_some((point.x, point.y))
}
pub(in crate::desktop::composition::live::window) unsafe fn handle(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    let terminal = matches!(
        message,
        WM_KILLFOCUS
            | WM_CANCELMODE
            | WM_ENTERMENULOOP
            | WM_SYSCOMMAND
            | WM_CLOSE
            | WM_DESTROY
            | WM_NCDESTROY
    ) || (message == WM_ACTIVATE && wp.0 & 0xffff == WA_INACTIVE as usize)
        || (message == WM_SIZE && wp.0 == SIZE_MINIMIZED as usize)
        || (message == WM_SHOWWINDOW && wp.0 == 0)
        || (message == WM_CAPTURECHANGED
            && state.drag.get().is_some()
            && HWND(lp.0 as *mut _) != hwnd);
    if terminal {
        cancel(hwnd, state);
        return false;
    }
    if state.drag.get().is_some() {
        if state.closed.get()
            || state.group_locked.get()
            || state.group_hidden.get()
            || GetCapture() != hwnd
        {
            cancel(hwnd, state);
            return false;
        }
        if matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN) && wp.0 == VK_ESCAPE.0 as usize {
            cancel(hwnd, state);
            return true;
        }
        if message == WM_LBUTTONUP {
            let owner = state.drag.get().map(|drag| drag.owner);
            update(hwnd, state, lp);
            if state.drag.get().map(|drag| drag.owner) == owner {
                cancel(hwnd, state);
            }
            return true;
        }
        if matches!(message, WM_RBUTTONUP | WM_MBUTTONUP) {
            return true;
        }
        if message == WM_MOUSEMOVE {
            update(hwnd, state, lp);
            return true;
        }
        // 不允许滚轮或第二个按钮在同一捕获事务中建立另一个本地几何操作。
        if matches!(
            message,
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL | WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN
        ) {
            return true;
        }
    }
    if message != WM_LBUTTONDOWN
        || (!pressed(VK_MENU) && !super::super::menu::corner(hwnd, state, lp))
        || pressed(VK_CONTROL)
        || (pressed(VK_SHIFT) && !pressed(VK_MENU))
        || pressed(VK_LWIN)
        || pressed(VK_RWIN)
    {
        return false;
    }
    begin_with(
        hwnd,
        state,
        lp,
        pressed(VK_MENU) && pressed(VK_SHIFT),
        || input::cancel(hwnd, state),
    )
}
unsafe fn begin_with(
    hwnd: HWND,
    state: &State,
    lp: LPARAM,
    resize: bool,
    retire_source: impl FnOnce(),
) -> bool {
    // 撤销源输入会同步重入；内层 begin 不能覆盖尚未完成的所有权移交。
    if state.drag_starting.replace(true) {
        return true;
    }
    retire_source();
    if !state.drag_starting.replace(false) {
        return true;
    }
    if state.closed.get() || state.group_locked.get() || state.group_hidden.get() {
        return true;
    }
    let (Ok(rect), Some(pointer)) = (client_rect(hwnd), screen_point(hwnd, lp)) else {
        return true;
    };
    let owner = state.drag_revision.get().wrapping_add(1);
    state.drag_revision.set(owner);
    state.drag.set(Some(Drag {
        rect,
        pointer,
        resize,
        owner,
    }));
    SetCapture(hwnd);
    if GetCapture() != hwnd {
        cancel(hwnd, state);
    }
    true
}
unsafe fn update(hwnd: HWND, state: &State, lp: LPARAM) {
    let (Some(drag), Some(pointer)) = (state.drag.get(), screen_point(hwnd, lp)) else {
        return;
    };
    let revision = state.drag_revision.get().wrapping_add(1);
    state.drag_revision.set(revision);
    let current = || {
        state.drag_revision.get() == revision
            && state.drag.get().is_some()
            && !state.closed.get()
            && !state.group_locked.get()
            && !state.group_hidden.get()
            && GetCapture() == hwnd
    };
    let delta = (
        i64::from(pointer.0) - i64::from(drag.pointer.0),
        i64::from(pointer.1) - i64::from(drag.pointer.1),
    );
    if drag.resize {
        if let Some(next) = crate::pin_geometry::resize_drag(drag.rect, delta) {
            if place_while(hwnd, next, current).is_err() && current() {
                cancel(hwnd, state);
            }
        }
        return;
    }
    let x = i64::from(drag.rect.x) + delta.0;
    let y = i64::from(drag.rect.y) + delta.1;
    let (Ok(x), Ok(y)) = (i32::try_from(x), i32::try_from(y)) else {
        return;
    };
    if place_while(hwnd, PinRect { x, y, ..drag.rect }, current).is_err() && current() {
        cancel(hwnd, state);
    }
}

#[cfg(test)]
#[path = "composition_live_drag_tests.rs"]
mod tests;
