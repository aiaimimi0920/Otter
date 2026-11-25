//! Background frozen-instantiation input; immutable sources travel with tasks, without HWNDs or implicit resource-path authorization.
use crate::desktop::session_candidate::PreparedRestore;
use crate::{
    session_library::{self, Kind},
    session_source::Source,
    workspace_id::UnitId,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Input {
    pub source: Source,
    pub kind: Kind,
    pub entry_id: String,
    pub unit: UnitId,
    /// Already converted from physical desktop position to session logical coordinates at request time.
    pub mouse: (f64, f64),
    /// Captured in the same UI request as the application restore credential; topology changes must still be rejected before publication.
    pub active: Vec<UnitId>,
}

pub(in crate::desktop) struct Append {
    pub source: Source,
    pub retained: crate::pixels::Pixels,
}

impl Input {
    pub(super) fn valid(&self) -> bool {
        self.source.len() <= crate::session_document::MAX_SESSION_BYTES
            && UnitId::parse(&self.entry_id).is_some()
            && self.mouse.0.is_finite()
            && self.mouse.1.is_finite()
    }
    pub(in crate::desktop) fn prepare(
        self,
        catalog: Option<&crate::loom_handshake::Handshake>,
        cancel: &AtomicBool,
    ) -> Result<(PreparedRestore, Append), String> {
        if cancel.load(Ordering::Acquire) {
            return Err("Frozen preparation cancelled".into());
        }
        let document = crate::session_document::decode(&self.source)
            .map_err(|error| format!("Frozen source rejected: {error:?}"))?;
        let library = document.library(self.kind)?;
        let entry = library
            .find(&self.entry_id)
            .ok_or("Frozen entry not found")?;
        let copy: Source = session_library::candidate::encode_selected(
            &document, entry, self.unit, self.mouse, cancel,
        )?
        .bytes
        .into();
        let parsed = copy
            .parse()
            .map_err(|error| format!("Invalid frozen candidate: {error:?}"))?;
        let retained = session_library::retention::prepare_decoded(
            &document,
            parsed.document(),
            &self.active,
            self.kind,
            &library,
            &self.entry_id,
            cancel,
        )?;
        // Retain the same budgeted encoded buffer without copying embedded images again for candidates; frozen sources have no external-file root.
        let restore = PreparedRestore::prepare_parsed(parsed, None, catalog, cancel)
            .map_err(|error| error.to_string())?;
        Ok((
            restore,
            Append {
                source: self.source,
                retained,
            },
        ))
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_frozen_input_tests.rs"]
mod tests;
