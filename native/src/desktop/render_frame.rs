//! One owner for D2D frames and local drawing state; failures/panics still end drawing and restore clip/transform.
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::*, *},
};

pub(super) fn draw(target: &ID2D1RenderTarget, paint: impl FnOnce() -> Result<()>) -> Result<()> {
    draw_observed(target, paint, || {})
}
/// Report only EndDraw failures; business-closure errors retain precedence without falsely marking the GPU invalid.
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
    // Preserve original drawing-error precedence; call EndDraw even if drawing returns Err.
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "render_frame_tests.rs"]
mod tests;
