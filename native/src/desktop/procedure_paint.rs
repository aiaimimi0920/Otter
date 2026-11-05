//! WM_PAINT owner：模型绘制、无借用软件提交、身份复核与原有错误策略分阶段执行。
use super::{paint_scope::PaintScope, State};
use windows::{
    core::Result,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

struct Cycle<'a> {
    state: &'a State,
    hwnd: HWND,
}
impl Drop for Cycle<'_> {
    fn drop(&mut self) {
        self.state.render_active.set(false);
        if self.state.render_deferred.replace(false) && self.state.lifetime.owns(self.hwnd) {
            unsafe {
                let _ = InvalidateRect(Some(self.hwnd), None, false);
            }
        }
    }
}
unsafe fn draw(hwnd: HWND, state: &State) -> Result<Option<super::super::canvas::Pending>> {
    let mut rect = RECT::default();
    GetClientRect(hwnd, &mut rect)?;
    let (width, height, dpi) = (
        (rect.right - rect.left).max(0) as u32,
        (rect.bottom - rect.top).max(0) as u32,
        GetDpiForWindow(hwnd),
    );
    let display = state.resolved_display.borrow();
    let mut canvas = state.canvas.borrow_mut();
    let normal = display.is_none() && canvas.minified.is_none();
    if display.is_some() {
        canvas.paint_display(hwnd, width, height, dpi, display.as_ref())?;
    } else {
        canvas.paint(hwnd, width, height, dpi)?;
    }
    Ok(if width > 0 && height > 0 {
        canvas.take_submission(normal)
    } else {
        None
    })
}
unsafe fn frame(hwnd: HWND, state: &State) -> Result<bool> {
    let owns = || state.lifetime.owns(hwnd);
    let allow_restore = || !state.group_hidden.get() && !super::super::tray::is_hidden(state);
    let mut removed_layered = false;
    if state.canvas.borrow().needs_gpu_style() {
        // 此处受 render_active 保护且不持 Canvas 借用；样式写入可重入换源/销毁。
        // 在新 GPU 可能关联前结束样式事务，attached failure 绝不能触发 layered 回滚。
        let style =
            super::super::software_surface::window_style::Transition::gpu_guarded(hwnd, &owns)?;
        removed_layered = style.removes_layered();
        style.commit()?;
    }
    let mut visibility = super::super::render_window_visibility::Reset::begin(
        hwnd,
        removed_layered,
        &owns,
        &allow_restore,
    )?;
    let result = draw_frame(hwnd, state);
    let restored = visibility.restore();
    match (result, restored) {
        (Err(error), Err(restore)) => Err(windows::core::Error::new(
            error.code(),
            format!("{error}; render visibility restore failed: {restore}"),
        )),
        (Err(error), _) => Err(error),
        (_, Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}
unsafe fn draw_frame(hwnd: HWND, state: &State) -> Result<bool> {
    // draw 返回时 Canvas 和 resolved_display 的所有 RefCell 借用已经结束。
    if let Some(pending) = draw(hwnd, state)? {
        if !super::software_present::submit(hwnd, state, pending)? {
            return Ok(false);
        }
    }
    if !state.lifetime.owns(hwnd) {
        return Ok(false);
    }
    super::super::hit_region::update(hwnd, state)?;
    Ok(true)
}
pub(super) unsafe fn paint(hwnd: HWND, state: &State) -> LRESULT {
    if state.render_active.replace(true) {
        // 嵌套 paint 只消费当前失效区，不创建另一后端、不清错误、不提前确认像素。
        state.render_deferred.set(true);
        drop(PaintScope::begin(hwnd));
        return LRESULT(0);
    }
    let _cycle = Cycle { state, hwnd };
    let scope = PaintScope::begin(hwnd);
    if state.canvas.borrow().creation_blocked() || state.canvas.borrow().render_suspended() {
        // 消费失效区但不清错误、不重建后端、不丢文档；没有自动解锁或定时重试。
        drop(scope);
        return LRESULT(0);
    }
    let result = frame(hwnd, state);
    drop(scope);
    if !state.lifetime.owns(hwnd) {
        return LRESULT(0);
    }
    match result {
        Ok(false) => {}
        Ok(true) => {
            state.error.borrow_mut().take();
            state.render_retries.set(0);
        }
        Err(error) => {
            if state.canvas.borrow().creation_blocked() {
                eprintln!("native-render: {error}; retry=false document=retained reason=unconfirmed-association");
                *state.error.borrow_mut() = Some(error);
                return LRESULT(0);
            }
            let downgraded = state.canvas.borrow_mut().recover_render_after(error.code());
            let recoverable = matches!(
                error.code(),
                D2DERR_RECREATE_TARGET
                    | windows::Win32::Graphics::Dxgi::DXGI_ERROR_DEVICE_REMOVED
                    | windows::Win32::Graphics::Dxgi::DXGI_ERROR_DEVICE_RESET
            );
            let retry = !downgraded
                && recoverable
                && state.render_retries.get() == 0
                && state.canvas.borrow_mut().retry_render_without_composition();
            eprintln!("native-render: {error}; asynchronous-recovery={downgraded}");
            *state.error.borrow_mut() = Some(error);
            if downgraded {
                state.render_retries.set(1);
                super::render_recovery::start(hwnd, state);
            } else if retry {
                state.render_retries.set(1);
                let _ = InvalidateRect(Some(hwnd), None, false);
            } else {
                state.canvas.borrow_mut().suspend_render();
            }
        }
    }
    LRESULT(0)
}
