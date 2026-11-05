//! 恢复缩略窗口的展开事务；物理坐标来自同一次恢复转换，不猜测历史 DPR。
use super::*;
use windows::core::{Error, Result};
use windows::Win32::Graphics::Gdi::SetWindowRgn;

pub(in crate::desktop) unsafe fn restore(hwnd: HWND, state: &State) -> Result<bool> {
    if !super::toggle_guard::allows(state) {
        return Ok(false);
    }
    let transition = {
        let canvas = state.canvas.borrow();
        let Some(presentation) = canvas.minified.as_ref() else {
            return Ok(false);
        };
        if !presentation.matches(&canvas.editor) {
            return Ok(false);
        }
        canvas.minified_restore
    };
    let Some(transition) = transition else {
        return Ok(false);
    };
    let opacity = transition.opacity;
    let mut current = RECT::default();
    GetWindowRect(hwnd, &mut current)?;
    let target = transition
        .target((current.left, current.top))
        .ok_or_else(|| Error::new(E_INVALIDARG, "Expanded window coordinates overflow"))?;
    let (presentation, previous_opacity) = {
        let mut canvas = state.canvas.borrow_mut();
        let opacity_before = canvas.window_opacity;
        canvas.window_opacity = opacity;
        (canvas.minified.take(), opacity_before)
    };
    // SetWindowPos 同步发送尺寸/绘制消息，必须先切换呈现并释放借用。
    if let Err(error) = SetWindowPos(
        hwnd,
        None,
        target.x,
        target.y,
        target.width as i32,
        target.height as i32,
        SWP_NOZORDER | SWP_NOACTIVATE,
    ) {
        let mut canvas = state.canvas.borrow_mut();
        canvas.minified = presentation;
        canvas.window_opacity = previous_opacity;
        return Err(error);
    }
    state.canvas.borrow_mut().minified_restore = None;
    fit(hwnd, state);
    let no_image = state.canvas.borrow().image.is_none();
    if no_image && SetWindowRgn(hwnd, None, false) == 0 {
        return Err(Error::from_win32());
    }
    super::super::hit_region::update(hwnd, state)?;
    let _ = InvalidateRect(Some(hwnd), None, false);
    Ok(true)
}
