//! 普通工作区与透明钉图共享 D3D12/D3D11/CPU 后端；窗口语义不隐式选择另一条 GPU 链。
use super::composition::Composition;
use super::composition_d3d12::CompositionD3d12;
#[path = "render_surface_selection.rs"]
mod selection;
#[path = "render_software.rs"]
mod software;
use windows::{
    core::Result,
    Win32::{
        Foundation::HWND,
        Graphics::Direct2D::{Common::D2D_SIZE_U, *},
    },
};

pub enum RenderSurface {
    D3d11(Box<Composition>),
    D3d12(Box<CompositionD3d12>),
    Software(Box<software::Software>),
}

impl RenderSurface {
    #[cfg(test)]
    pub(in crate::desktop) fn readback_for_test(
        &self,
    ) -> crate::desktop::presentation_readback_test::Pixels {
        match self {
            Self::D3d12(s) => s.readback_for_test(),
            Self::D3d11(s) => s.readback_for_test(),
            Self::Software(s) => s.readback_for_test(),
        }
    }
    /// 只退休本项目拥有的 DComp 图；None 表示此 surface 没有该图，不代表 GPU drain。
    /// 调用后由窗口事务保留 owner 并轮询；本方法不修改 HWND 样式、不自动切换后端。
    pub fn retire_composition(&mut self) -> Option<super::composition_retirement::Status> {
        match self {
            Self::D3d12(composition) => Some(composition.retire_composition()),
            Self::D3d11(composition) => Some(composition.retire_composition()),
            Self::Software(_) => None,
        }
    }
    pub fn set_opacity(&mut self, opacity: f32) -> Result<()> {
        match self {
            Self::Software(surface) => surface.set_opacity(opacity),
            Self::D3d12(composition) => composition.set_opacity(opacity),
            Self::D3d11(composition) => composition.set_opacity(opacity),
        }
    }
    pub fn new(
        factory: &ID2D1Factory,
        hwnd: HWND,
        size: D2D_SIZE_U,
        dpi: u32,
        pin: bool,
    ) -> super::composition_creation::Result<Self> {
        selection::select(
            || {
                CompositionD3d12::new_candidate(hwnd, size)
                    .map(|value| Self::D3d12(Box::new(value)))
            },
            || Self::legacy(factory, hwnd, size, dpi, pin),
            |surface| {
                if surface.is_software() {
                    if pin {
                        "software-pin"
                    } else {
                        "software-workspace"
                    }
                } else if pin {
                    "d3d11-pin"
                } else {
                    "d3d11-workspace"
                }
            },
        )
    }
    pub(super) fn legacy(
        factory: &ID2D1Factory,
        hwnd: HWND,
        size: D2D_SIZE_U,
        _dpi: u32,
        pin: bool,
    ) -> super::composition_creation::Result<Self> {
        // 两类窗口共享同一 D3D11 owner 和关联准入；不再以隐式 HWND target 充当兼容后端。
        let gpu = super::smoke_timing::run("compatibility-surface", || {
            Composition::new_candidate(hwnd, size)
        })
        .map(|surface| Self::D3d11(Box::new(surface)));
        selection::compatibility(gpu, pin, || Self::software_mode(factory, size, pin))
    }

    #[cfg(test)]
    pub(super) fn software(factory: &ID2D1Factory, size: D2D_SIZE_U) -> Result<Self> {
        Self::software_mode(factory, size, false)
    }
    pub(super) fn software_mode(
        factory: &ID2D1Factory,
        size: D2D_SIZE_U,
        pinned: bool,
    ) -> Result<Self> {
        software::Software::new(factory, size, pinned)
            .map(|surface| Self::Software(Box::new(surface)))
    }
    pub(super) fn requires_software(&self) -> bool {
        matches!(self, Self::D3d11(surface) if surface.requires_rebuild())
    }
    pub(super) fn is_software(&self) -> bool {
        matches!(self, Self::Software(_))
    }
    pub(super) fn take_submission(&mut self) -> Option<super::software_surface::Submission> {
        match self {
            Self::Software(surface) => surface.take_submission(),
            _ => None,
        }
    }

    pub(super) fn requires_compatibility(&self) -> bool {
        matches!(self, Self::D3d12(surface) if surface.requires_rebuild())
    }
    pub(super) fn is_compatibility(&self) -> bool {
        !matches!(self, Self::D3d12(_))
    }

    /// 仅用于创建/同步同设备画刷和位图；绘制与提交必须走 draw 的单一事务。
    pub fn resource_target(&self) -> &ID2D1RenderTarget {
        match self {
            Self::Software(surface) => surface.resource_target(),
            Self::D3d12(composition) => composition.resource_target(),
            Self::D3d11(composition) => composition.renderer(),
        }
    }

    pub fn resize(&mut self, size: D2D_SIZE_U) -> Result<()> {
        match self {
            Self::Software(surface) => surface.resize(size),
            Self::D3d12(composition) => composition.resize(size),
            Self::D3d11(composition) => composition.resize(size),
        }
    }

    pub fn draw(&mut self, paint: impl FnOnce(&ID2D1RenderTarget) -> Result<()>) -> Result<bool> {
        // 后端独占 BeginDraw/EndDraw 和提交；外层不能再套第二个 D2D 帧。
        match self {
            Self::Software(surface) => surface.draw(paint),
            Self::D3d12(composition) => {
                composition.draw(paint)?;
                composition.present()
            }
            Self::D3d11(composition) => {
                composition.draw(paint)?;
                composition.present_confirmed()
            }
        }
    }
}
