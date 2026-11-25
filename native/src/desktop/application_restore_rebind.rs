//! Capability catalogs arriving after restoration bind only Art not yet selected by the user, without running tasks or changing images/source content.
use super::*;
use crate::desktop::{
    capability_selection::Selection, document_state, session_binding::catalog_matches,
};
use crate::session_art_batch::{resolve_optional, Binding};
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_restore_rebind_smoke.rs"]
pub(super) mod smoke;

impl Application {
    pub(in crate::desktop) fn rebind_restored_arts(&mut self) -> Result<()> {
        if self.modals.blocked() || self.exit_requested.get() || self.restore_job.is_some() {
            return Ok(());
        }
        let Some(catalog) = self.catalog.borrow().loaded.clone() else {
            return Ok(());
        };
        let Some(record) = self.restored_session.as_mut() else {
            return Ok(());
        };
        if !catalog_matches(&record.binding_catalog, Some(&catalog)) {
            record.binding_catalog = Some(catalog.clone());
            record.attempted_arts.clear();
        }
        // User selection or closure permanently retires this restore candidate; do not reapply old session settings to the current node.
        record.pending_arts.retain(|unit| {
            self.documents.iter().any(|doc| {
                doc.state.unit_id == *unit
                    && doc.state.restored_art
                    && doc.state.selected_art.borrow().is_none()
            })
        });
        let eligible: Vec<_> = record
            .pending_arts
            .iter()
            .filter(|unit| {
                !record.attempted_arts.contains(unit)
                    && self.documents.iter().any(|doc| {
                        doc.state.unit_id == **unit
                            && !crate::desktop::group_lock::editing_busy(&doc.state)
                            && !doc.state.file_dialog.get()
                            && doc.state.pending_adoption.borrow().is_none()
                            && doc.state.pending_start.borrow().is_none()
                            && doc.state.pending_execution.borrow().is_none()
                    })
            })
            .cloned()
            .collect();
        if eligible.is_empty() {
            return Ok(());
        }
        // Try each candidate only once per catalog; errors preserve source content, which a later catalog version may resolve again.
        record.attempted_arts.extend(eligible.iter().cloned());
        let mut document = crate::session_document::decode(&record.original)
            .map_err(|_| Error::new(E_INVALIDARG, "Retained session cannot be rebound"))?;
        document
            .windows
            .retain(|window| eligible.iter().any(|unit| unit.as_str() == window.id));
        let entries = resolve_optional(
            &document,
            Some(&catalog.handshake),
            &std::sync::atomic::AtomicBool::new(false),
        )
        .map_err(|_| {
            Error::new(
                E_INVALIDARG,
                "Restored Art parameters rejected by current capability catalog",
            )
        })?;
        let mut prepared = Vec::new();
        for entry in entries {
            if let Binding::Ready { restored, .. } = entry.binding {
                prepared.push((
                    entry.unit,
                    Selection {
                        identity: document_state::new_unit_id()?,
                        catalog: catalog.clone(),
                        parameters: restored.parameters,
                        execution: restored.execution,
                    },
                ));
            }
        }
        // Install everything only after all resolution and identity allocation succeed; this function pumps no messages and allows no intervening UI reentry.
        for (unit, selection) in prepared {
            if let Some(doc) = self.documents.iter().find(|doc| doc.state.unit_id == unit) {
                *doc.state.selected_art.borrow_mut() = Some(selection);
                record.pending_arts.retain(|pending| *pending != unit);
            }
        }
        self.display_cache.borrow_mut().take();
        Ok(())
    }
}
