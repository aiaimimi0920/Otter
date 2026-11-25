//! Content styles reuse workflow inner strokes/budgeted rounded layers; logical window pixels do not scale with image viewports.
use super::export::source_resolution::{frame_border::Border, frame_clip};
use crate::{crop_geometry::Rect, pin_frame_style::Style};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::D2D_SIZE_U, ID2D1Factory, ID2D1RenderTarget},
};

pub(super) struct Frame {
    style: Style,
    rect: Rect,
    output: (u32, u32),
}
impl Frame {
    pub(super) fn new(style: Style, size: D2D_SIZE_U, dpi: u32) -> Self {
        Self {
            style,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: f64::from(size.width) * 96.0 / f64::from(dpi),
                height: f64::from(size.height) * 96.0 / f64::from(dpi),
            },
            output: (size.width, size.height),
        }
    }
    pub(super) unsafe fn clip<'a>(
        &self,
        target: &'a ID2D1RenderTarget,
        factory: &ID2D1Factory,
    ) -> Result<Option<frame_clip::Clip<'a>>> {
        frame_clip::push_frame(
            target,
            factory,
            self.rect,
            self.style.radius(),
            self.output,
            crate::pixels::reserve,
        )
    }
    pub(super) unsafe fn border(&self, target: &ID2D1RenderTarget) -> Result<()> {
        if let Some(border) = Border::from_style(self.style) {
            border.draw(target, self.rect)?;
        }
        Ok(())
    }
    /// Callers hold exactly one rectangular content clip; inner strokes belong to the full window and must not be cropped by contain padding.
    pub(super) unsafe fn border_over_clip(
        &self,
        target: &ID2D1RenderTarget,
        clip: &windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F,
    ) -> Result<()> {
        if self.style.border().is_none() {
            return Ok(());
        }
        target.PopAxisAlignedClip();
        let result = self.border(target);
        target.PushAxisAlignedClip(
            clip,
            windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED,
        );
        result
    }
}
