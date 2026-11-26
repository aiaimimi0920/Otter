//! Ordinary canvases prefer the shared D3D11 device; Tile retains its D3D12-first presentation contract.
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::desktop) enum Primary {
    SharedD3d11,
    D3d12,
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
    /// Retire only this project's owned DComp graph; None means this surface lacks that graph, not GPU drain.
    /// The window transaction retains and polls the owner afterward; this method neither changes HWND styles nor switches backends automatically.
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
        primary: Primary,
    ) -> super::composition_creation::Result<Self> {
        if primary == Primary::SharedD3d11 {
            return Self::d3d11_automatic(factory, hwnd, size, dpi, pin);
        }
        selection::select(
            || {
                CompositionD3d12::new_candidate(hwnd, size)
                    .map(|value| Self::D3d12(Box::new(value)))
            },
            || Self::d3d11_automatic(factory, hwnd, size, dpi, pin),
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
    pub(super) fn d3d11_automatic(
        factory: &ID2D1Factory,
        hwnd: HWND,
        size: D2D_SIZE_U,
        _dpi: u32,
        pin: bool,
    ) -> super::composition_creation::Result<Self> {
        let gpu =
            super::smoke_timing::run("d3d11-surface", || Composition::new_candidate(hwnd, size))
                .map(|surface| Self::D3d11(Box::new(surface)));
        selection::compatibility(gpu, pin, || Self::software_mode(factory, size, pin))
    }
    pub(super) fn legacy(
        factory: &ID2D1Factory,
        hwnd: HWND,
        size: D2D_SIZE_U,
        _dpi: u32,
        pin: bool,
    ) -> super::composition_creation::Result<Self> {
        // Explicit Compatibility tier uses D3D11 WARP so hardware driver bugs step down to software rasterized D3D11 before GDI.
        let gpu = super::smoke_timing::run("compatibility-surface", || {
            Composition::new_warp_candidate(hwnd, size)
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
    #[cfg(test)]
    pub(super) fn is_d3d11_warp(&self) -> bool {
        matches!(self, Self::D3d11(surface) if surface.is_warp())
    }
    pub(super) fn take_submission(&mut self) -> Option<super::software_surface::Submission> {
        match self {
            Self::Software(surface) => surface.take_submission(),
            _ => None,
        }
    }

    pub(super) fn requires_compatibility(&self) -> bool {
        match self {
            Self::D3d12(surface) => surface.requires_rebuild(),
            Self::D3d11(surface) => !surface.is_warp() && surface.requires_rebuild(),
            Self::Software(_) => false,
        }
    }
    pub(super) fn is_compatibility(&self) -> bool {
        match self {
            Self::D3d12(_) => false,
            Self::D3d11(surface) => surface.is_warp(),
            Self::Software(_) => true,
        }
    }

    /// Only for creating/synchronizing same-device brushes and bitmaps; drawing and submission must use draw's single transaction.
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
        // The backend exclusively owns BeginDraw/EndDraw and submission; the outer layer must not wrap another D2D frame around it.
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
