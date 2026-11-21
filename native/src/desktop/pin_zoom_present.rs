//! 滚轮呈现事务：GPU 在客户区几何提交前准备新帧，CPU 一次提交像素和目标矩形。
use super::{PinRect, State};
use std::cell::Cell;
use windows::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};

#[derive(Default)]
pub(in crate::desktop) struct Resize {
    pub rect: Cell<Option<PinRect>>,
    // 表示原绘制链无错误完成，不替代 confirmed_paints，更不代表 DWM 已显示；遮挡沿原重绘语义。
    pub prepared: Cell<bool>,
}

struct Scope<'a>(&'a Resize);
impl Drop for Scope<'_> {
    fn drop(&mut self) {
        self.0.rect.set(None);
        self.0.prepared.set(false);
    }
}

pub(in crate::desktop) unsafe fn apply(hwnd: HWND, state: &State, after: PinRect) -> bool {
    // 不能在绘制或另一 resize 的重入中替换待提交几何。
    if state.render_active.get() || state.pin_resize.rect.get().is_some() {
        return false;
    }
    state.pin_resize.rect.set(Some(after));
    state.pin_resize.prepared.set(false);
    let scope = Scope(&state.pin_resize);
    // region 同时裁剪可见像素：旧 region 随新原点移动会让右边缘反向跳动。
    // 同步栈内解除旧裁剪，返回输入泵前恢复精确 alpha 区域。
    if !super::preview::active(state) && SetWindowRgn(hwnd, None, false) == 0 {
        *state.error.borrow_mut() = Some(windows::core::Error::from_win32());
        return false;
    }
    if !state.lifetime.owns(hwnd) {
        return false;
    }
    super::fit_size(hwnd, state, (after.width as i32, after.height as i32));
    crate::desktop::application::chrome::zoom_frame(&state.chrome, hwnd, after);
    if !state.lifetime.owns(hwnd) {
        return false;
    }
    let software = state.canvas.borrow().software_selected();
    let positioned = if software && !super::preview::active(state) {
        // 不先 SetWindowPos 搬动旧位图；ULW_ALPHA 使用本事务的目标位置和新像素。
        crate::desktop::procedure::resize_frame(hwnd, state);
        state.pin_resize.prepared.get()
    } else {
        // NCCALCSIZE 在窗口几何生效之前交付；不要等 WM_SIZE 后才 ResizeBuffers/Present。
        SetWindowPos(
            hwnd,
            None,
            after.x,
            after.y,
            after.width as i32,
            after.height as i32,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS | SWP_NOREDRAW,
        )
        .is_ok()
    };
    let prepared = state.pin_resize.prepared.get();
    if !state.lifetime.owns(hwnd) {
        return false;
    }
    super::fit(hwnd, state);
    // 成功和失败都恢复实际客户区的精确 alpha 区域，不能留下矩形命中范围。
    // resize 范围保持到区域恢复完成，使缓存不能跳过撤销临时无裁剪状态。
    let region = crate::desktop::hit_region::update(hwnd, state);
    drop(scope);
    if !state.lifetime.owns(hwnd) {
        return false;
    }
    crate::desktop::application::chrome::geometry(&state.chrome, hwnd, WM_WINDOWPOSCHANGED);
    if let Err(error) = region {
        *state.error.borrow_mut() = Some(error);
        return false;
    }
    if positioned && prepared {
        // 几何提交时 WM_SIZE 的失效区已由同一新尺寸帧覆盖，不再重复画一帧。
        let _ = ValidateRect(Some(hwnd), None);
    } else {
        let _ = InvalidateRect(Some(hwnd), None, false);
        let _ = UpdateWindow(hwnd);
    }
    // 缩放累积跟随实际 HWND 几何。细步进可能只改变取整后的位置、不发 NCCALCSIZE；
    // 此时虽走常规补绘，也不能把已成功定位的亚像素累积清零。
    positioned
}

pub(in crate::desktop) unsafe fn before_geometry(
    hwnd: HWND,
    state: &State,
    wp: WPARAM,
    lp: LPARAM,
) {
    if super::preview::active(state) {
        return;
    }
    if wp.0 == 0 || lp.0 == 0 || state.pin_resize.prepared.get() || state.render_active.get() {
        return;
    }
    let Some(rect) = state.pin_resize.rect.get() else {
        return;
    };
    let client = (*(lp.0 as *const NCCALCSIZE_PARAMS)).rgrc[0];
    // 只处理本次普通无边框钉图的预期客户区；其他布局/DPI 消息沿原路径。
    if client.right - client.left == rect.width as i32
        && client.bottom - client.top == rect.height as i32
        && !state.canvas.borrow().software_selected()
    {
        crate::desktop::procedure::resize_frame(hwnd, state);
    }
}
