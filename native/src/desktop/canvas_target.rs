//! 将已创建 surface 的资源准备纳入同一准入事务；失败不得绕过合成退休 owner。
use super::Target;
use crate::desktop::{composition_creation, render_surface::RenderSurface};
use windows::{core::Result, Win32::Graphics::Direct2D::*};

impl Target {
    pub(super) fn prepare(
        mut surface: RenderSurface,
        brush: impl FnOnce(&ID2D1RenderTarget) -> Result<ID2D1SolidColorBrush>,
    ) -> composition_creation::Result<Self> {
        let brush = match brush(surface.resource_target()) {
            Ok(brush) => brush,
            Err(error) => {
                // 仍持有完整 surface 才发起退休；有 DComp 图即保守拒绝立即接管。
                // None 只表示没有本项目 DComp 图，不代表 GPU drain。
                return Err(if surface.retire_composition().is_some() {
                    composition_creation::Failure::attached(error)
                } else {
                    error.into()
                });
            }
        };
        Ok(Self {
            identity: std::rc::Rc::new(()),
            effects: Default::default(),
            raster: Default::default(),
            brush,
            surface,
        })
    }
}
