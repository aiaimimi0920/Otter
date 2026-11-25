//! Frozen restoration/reference copying reuse the application's single-task restore owner and retain the same source and coordinate credentials throughout background work.
use super::*;
use crate::{session_library::Kind, session_source::Source, workspace_id::UnitId};

impl Application {
    /// Call after leaving the menu's modal scope; source is the same session owner retained when the menu listed its entries.
    /// This entry point does not authorize external files. Embedded images can be restored; existing resource admission still rejects other resources.
    pub(in crate::desktop::application) unsafe fn request_frozen(
        &mut self,
        source: Source,
        kind: Kind,
        entry_id: String,
        physical_mouse: POINT,
    ) -> Result<UnitId> {
        let record = self
            .restored_session
            .as_ref()
            .ok_or_else(|| Error::new(E_ABORT, "Missing frozen session source"))?;
        if !record.original.same_storage(&source) || UnitId::parse(&entry_id).is_none() {
            return Err(Error::new(
                E_ABORT,
                "Frozen menu source is stale or invalid",
            ));
        }
        let space = crate::session_coordinates::select(record.space, None)
            .map_err(|message| Error::new(E_INVALIDARG, message))?;
        let unit = crate::desktop::document_state::new_unit_id()?;
        let mouse = (
            (physical_mouse.x as f64 - space.origin.0 as f64) / space.scale,
            (physical_mouse.y as f64 - space.origin.1 as f64) / space.scale,
        );
        let input = session_job::Input::Frozen(session_job::frozen::Input {
            source,
            kind,
            entry_id,
            unit: unit.clone(),
            mouse,
            active: self.workspace.units().to_vec(),
        });
        // Reuse save/restore exclusion, the startup guard, catalog snapshot, wakeups, cancellation, and exit reaping.
        self.start_restore(input, None, Some(space))?;
        Ok(unit)
    }
}

#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_frozen_job_smoke.rs"]
pub(super) mod smoke;
