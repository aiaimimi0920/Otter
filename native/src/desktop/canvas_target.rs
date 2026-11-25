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
