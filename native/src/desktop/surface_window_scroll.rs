//! 本地滚轮路由；读取系统偏好，屏幕物理坐标转客户区，不产生 Surface 动作。
use super::*;
pub(super) unsafe fn message(
    hwnd: HWND,
    state: &State,
    message: u32,
    wp: WPARAM,
    lp: LPARAM,
) -> bool {
    if !state.local_edit.get() || state.closed.get() {
        return false;
    }
    let Some(input) = crate::desktop::surface_scroll_input::wheel(hwnd, message, wp, lp) else {
        return false;
    };
    let mut changed = false;
    let result = state.run(|host| {
        changed = host.scroll(input.pixel, input.axis, input.delta, input.page)?;
        Ok(())
    });
    if let Err(error) = result {
        // EDIT 重定位可能在呈现提交后失败；失败同样废弃旧几何上的输入候选。
        pointer::invalidate(hwnd, state);
        let _ = InvalidateRect(Some(hwnd), None, false);
        *state.error.borrow_mut() = Some(error);
        return true;
    }
    if changed {
        // 几何改变后，旧按下/待发点击不能重新命中新位置的节点。
        pointer::invalidate(hwnd, state);
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    changed
}
