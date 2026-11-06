//! 系统移动使用原宿主输入循环，只有可见图框整帧代理；不建立第二套鼠标捕获。
use super::{PinRect, State};
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

pub(super) unsafe fn begin(hwnd: HWND, state: &State) {
    let eligible = {
        let canvas = state.canvas.borrow();
        canvas.pinned
            && canvas.tool == crate::editor::Tool::Pan
            && !canvas.blocks_wheel()
            && (canvas.image.is_some() || state.resolved_display.borrow().is_some())
    };
    if !eligible || !IsWindowVisible(hwnd).as_bool() {
        return;
    }
    let mut rect = RECT::default();
    if let Err(error) = GetWindowRect(hwnd, &mut rect).and_then(|_| {
        super::preview::begin(
            hwnd,
            state,
            PinRect {
                x: rect.left,
                y: rect.top,
                width: (rect.right - rect.left).max(1) as u32,
                height: (rect.bottom - rect.top).max(1) as u32,
            },
        )
    }) {
        *state.error.borrow_mut() = Some(error);
    }
}

pub(in crate::desktop) unsafe fn position(hwnd: HWND, state: &State, rect: PinRect) {
    if (!super::animation::moving(state) && !super::stationary::active(state))
        || !super::preview::active(state)
    {
        return;
    }
    if !super::preview::fresh(state) {
        super::animation::stop(hwnd, state);
        return;
    }
    if let Err(error) = super::preview::move_cached(hwnd, state, rect) {
        super::animation::stop(hwnd, state);
        *state.error.borrow_mut() = Some(error);
    }
}
