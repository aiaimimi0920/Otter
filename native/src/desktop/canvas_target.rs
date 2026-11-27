//! Include resource preparation for created surfaces in the same admission transaction; failures must not bypass composition-retirement ownership.
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
                // Start retirement while still holding the complete surface; a DComp graph conservatively rules out immediate takeover.
                // None means only that no project-owned DComp graph exists, not that GPU drain completed.
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

impl super::Canvas {
    pub(super) unsafe fn ensure_target(
        &mut self,
        hwnd: windows::Win32::Foundation::HWND,
        size: Common::D2D_SIZE_U,
        dpi: u32,
    ) -> Result<()> {
        if self.target.is_some() {
            return Ok(());
        }
        let prepared = self.creation_admission.attempt(|| {
            let surface = if self.software_renderer {
                RenderSurface::software_mode(&self.factory, size, self.pinned).map_err(Into::into)
            } else if self.compatibility_renderer {
                RenderSurface::legacy(&self.factory, hwnd, size, dpi, self.pinned)
            } else {
                RenderSurface::new(&self.factory, hwnd, size, dpi, self.pinned, self.primary)
            }?;
            Target::prepare(surface, |t| {
                t.CreateSolidColorBrush(
                    &Common::D2D1_COLOR_F {
                        r: 0.96,
                        g: 0.88,
                        b: 0.24,
                        a: 1.,
                    },
                    None,
                )
            })
        })?;
        if !matches!(prepared.surface, RenderSurface::D3d12(_)) {
            self.primary = crate::desktop::render_surface::Primary::SharedD3d11;
        }
        self.compatibility_renderer |= prepared.surface.is_compatibility();
        self.software_renderer |= prepared.surface.is_software();
        self.target = Some(prepared);
        Ok(())
    }
}
