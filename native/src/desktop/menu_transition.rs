//! 菜单边界的手势收尾与呈现同步；命令目录不复制捕获释放和钉图命中区域更新。
use super::State;
use windows::{
    core::Result,
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::InvalidateRect,
        UI::{
            Input::KeyboardAndMouse::{GetCapture, ReleaseCapture},
            WindowsAndMessaging::IsWindow,
        },
    },
};

pub(super) unsafe fn prepare(hwnd: HWND, state: &State) -> Result<()> {
    state.canvas.borrow_mut().cancel_gesture();
    if GetCapture() == hwnd {
        ReleaseCapture()?;
    }
    Ok(())
}

pub(super) unsafe fn finish(hwnd: HWND, state: &State) -> Result<()> {
    if IsWindow(Some(hwnd)).as_bool() {
        super::tool_panel::invalidate(state);
        super::crop_window::sync(hwnd, state)?;
        super::hit_region::update(hwnd, state)?;
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    Ok(())
}
