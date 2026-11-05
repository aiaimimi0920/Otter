//! Canvas 软件适配：只生成离屏帧，窗口提交由释放模型借用后的 paint owner 执行。
use crate::desktop::software_surface::{SoftwareSurface, Submission};
use windows::{
    core::Result,
    Win32::Graphics::Direct2D::{Common::D2D_SIZE_U, *},
};

pub struct Software {
    surface: SoftwareSurface,
    pending: Option<Submission>,
    opacity: f32,
    pinned: bool,
}
impl Software {
    #[cfg(test)]
    pub(in crate::desktop) fn readback_for_test(
        &self,
    ) -> crate::desktop::presentation_readback_test::Pixels {
        assert!(self.pending.is_none());
        self.surface.readback_for_test()
    }
    pub fn new(factory: &ID2D1Factory, size: D2D_SIZE_U, pinned: bool) -> Result<Self> {
        Ok(Self {
            surface: SoftwareSurface::new(factory, size)?,
            pending: None,
            opacity: 1.0,
            pinned,
        })
    }
    pub fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        super::super::software_surface::validate_presentation(self.pinned, opacity)?;
        self.opacity = opacity;
        Ok(())
    }
    pub fn resource_target(&self) -> &ID2D1RenderTarget {
        self.surface.resource_target()
    }
    pub fn resize(&mut self, size: D2D_SIZE_U) -> Result<()> {
        self.surface.resize(size)
    }
    pub fn draw(&mut self, paint: impl FnOnce(&ID2D1RenderTarget) -> Result<()>) -> Result<bool> {
        self.pending = None; // 未取走的旧帧取消；绝不重新提交旧像素。
        self.surface.draw(paint)?;
        self.pending = Some(self.surface.prepare_present(self.pinned, self.opacity)?);
        Ok(false) // 离屏成功不代表 HWND 已呈现。
    }
    pub fn take_submission(&mut self) -> Option<Submission> {
        self.pending.take()
    }
}
