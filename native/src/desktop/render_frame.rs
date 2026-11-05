//! D2D 帧和局部绘制状态的单一 owner；失败或 panic 同样结束绘制并恢复 clip/transform。
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) fn draw(target: &ID2D1RenderTarget, paint: impl FnOnce() -> Result<()>) -> Result<()> {
    draw_observed(target, paint, || {})
}
/// 只报告 EndDraw 失败；业务闭包错误保持原优先级，不误标记 GPU 失效。
pub(super) fn draw_observed(
    target: &ID2D1RenderTarget,
    paint: impl FnOnce() -> Result<()>,
    failed: impl FnOnce(),
) -> Result<()> {
    unsafe {
        target.BeginDraw();
    }
    let frame = Drawing {
        target,
        active: true,
    };
    let painted = paint();
    let ended = frame.end();
    if ended.is_err() {
        failed();
    }
    // 保持原绘制错误优先级；即使绘制返回 Err，仍须调用 EndDraw。
    painted?;
    ended
}
struct Drawing<'a> {
    target: &'a ID2D1RenderTarget,
    active: bool,
}
impl Drawing<'_> {
    fn end(mut self) -> Result<()> {
        self.active = false;
        unsafe { self.target.EndDraw(None, None) }
    }
}
impl Drop for Drawing<'_> {
    fn drop(&mut self) {
        if self.active {
            unsafe {
                let _ = self.target.EndDraw(None, None);
            }
        }
    }
}

pub(super) struct Clip<'a>(&'a ID2D1RenderTarget);
impl<'a> Clip<'a> {
    pub fn push(target: &'a ID2D1RenderTarget, rect: &D2D_RECT_F) -> Self {
        unsafe {
            target.PushAxisAlignedClip(rect, D2D1_ANTIALIAS_MODE_ALIASED);
        }
        Self(target)
    }
}
impl Drop for Clip<'_> {
    fn drop(&mut self) {
        unsafe {
            self.0.PopAxisAlignedClip();
        }
    }
}

pub(super) fn save_transform(target: &ID2D1RenderTarget) -> impl Drop + '_ {
    let mut original = Default::default();
    unsafe {
        target.GetTransform(&mut original);
    }
    Transform(Some(move || unsafe {
        target.SetTransform(&original);
    }))
}
struct Transform<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for Transform<F> {
    fn drop(&mut self) {
        if let Some(restore) = self.0.take() {
            restore();
        }
    }
}

#[cfg(test)]
#[path = "render_frame_tests.rs"]
mod tests;
