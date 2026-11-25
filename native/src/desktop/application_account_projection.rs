//! v2 single-sticker takeover reuses real Bound/append/checkpoint paths; network acceptance, disk persistence, and visible presentation settle separately.
use super::*;
use crate::desktop::session_candidate::PreparedRestore;
use crate::projection_account::scene::Binding;
#[cfg(test)]
use crate::projection_account::scene::Index;
use std::sync::atomic::AtomicBool;
pub(super) struct Prepared {
    pub restore: PreparedRestore,
    pub retained: crate::pixels::Pixels,
    pub binding: Binding,
}
/// Original workspace identity captured before network I/O; this is not authorization and never reselects an anchor or current scene from a late response.
pub(super) struct Destination {
    source: crate::session_source::Source,
    workspace: crate::workspace::IdentityStamp,
    guard: restore_batch::guard::Guard,
    consent: Option<crate::desktop::device_authorization_ui::policy::Stamp>,
    participation: crate::desktop::device_authorization_ui::policy::Participation,
    pub space: crate::session_placement::Space,
    pub scope: crate::session_projection_scope::Scope,
}
impl Destination {
    pub(super) fn source(&self) -> &crate::session_source::Source {
        &self.source
    }
    pub(super) unsafe fn capture(app: &Application) -> Result<Self> {
        Self::capture_for(app, false)
    }
    pub(super) unsafe fn capture_existing(app: &Application) -> Result<Self> {
        Self::capture_for(app, true)
    }
    unsafe fn capture_for(app: &Application, existing: bool) -> Result<Self> {
        if !app.projection_checkpoint_available()
            || app.restore_job.is_some()
            || app.save_job.is_some()
            || app.exit_requested.get()
            || app.modals.blocked()
            || app.workspace.units().len() >= crate::workspace::MAX_UNITS
        {
            return Err(stale());
        }
        let record = app.restored_session.as_ref().ok_or_else(stale)?;
        Ok(Self {
            source: record.original.clone(),
            workspace: app.workspace.identity_stamp(),
            guard: restore_batch::guard::Guard::capture(app)?,
            consent: if existing {
                None
            } else {
                Some(
                    app.authorization
                        .borrow()
                        .consent
                        .stamp()
                        .ok_or_else(stale)?,
                )
            },
            participation: app
                .authorization
                .borrow()
                .consent
                .participation()
                .ok_or_else(stale)?,
            space: record.space.ok_or_else(stale)?,
            scope: record.projection_scope.ok_or_else(stale)?,
        })
    }
    pub(super) unsafe fn matches(&self, app: &Application) -> bool {
        app.projection_checkpoint_available()
            && app.restore_job.is_none()
            && app.save_job.is_none()
            && !app.exit_requested.get()
            && !app.modals.blocked()
            && self.workspace.matches(&app.workspace)
            && self.guard.matches(app)
            && self
                .consent
                .as_ref()
                .is_none_or(|stamp| app.authorization.borrow().consent.matches(stamp))
            && app
                .authorization
                .borrow()
                .consent
                .participates(&self.participation)
            && app.restored_session.as_ref().is_some_and(|record| {
                record.original.same_storage(&self.source)
                    && record.projection_scope == Some(self.scope)
                    && restore_batch::append::same_space(record.space, self.space)
            })
    }
    /// Tests assemble this from real background append results; still check account scope and bindings in the candidate.
    #[cfg(test)]
    pub(super) fn prepare(
        &self,
        restore: PreparedRestore,
        binding: Binding,
        retained: crate::pixels::Pixels,
    ) -> Result<Prepared> {
        if binding.scope() != self.scope {
            return Err(stale());
        }
        let index = Index::capture(&restore.vector.candidate.retained_source_record())
            .map_err(|_| stale())?;
        if !index.includes(&binding) {
            return Err(stale());
        }
        Ok(Prepared {
            restore,
            retained,
            binding,
        })
    }
}
pub(super) struct Receipt {
    pub binding: Binding,
    workspace: crate::workspace::IdentityStamp,
    source: crate::session_source::Source,
    consent: Option<crate::desktop::device_authorization_ui::policy::Stamp>,
    participation: crate::desktop::device_authorization_ui::policy::Participation,
}
impl Receipt {
    /// Saving cannot restore presentation permission from before panel closure, pause, or a source change; the disk receipt remains valid.
    pub(super) unsafe fn present(
        &self,
        app: &Application,
        cancel: Option<&AtomicBool>,
    ) -> Result<bool> {
        let view = app.authorization.borrow();
        let allowed = !view.closed
            && view.origin.as_deref() == Some(self.binding.origin())
            && view.consent.participates(&self.participation)
            && self
                .consent
                .as_ref()
                .is_none_or(|stamp| view.consent.matches(stamp));
        drop(view);
        if !allowed
            || cancel.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire))
            || !self.saved(app)
        {
            return Ok(false);
        }
        let doc = app
            .documents
            .iter()
            .find(|doc| doc.state.unit_id.as_str() == self.binding.unit())
            .ok_or_else(stale)?;
        projection::receive::present::Visual::capture(doc)?.present(doc)?;
        Ok(true)
    }
    /// Query only the background index installed at actual commit; do not scan sessions, access files, or verify signatures on the UI thread.
    pub(super) fn saved(&self, app: &Application) -> bool {
        self.workspace.matches(&app.workspace)
            && app
                .restored_session
                .as_ref()
                .is_some_and(|record| record.original.same_storage(&self.source))
            && app
                .workspace_checkpoint
                .as_ref()
                .is_some_and(|owner| owner.includes_account(&self.binding, &app.workspace))
    }
}
impl Application {
    pub(super) unsafe fn publish_account_projection(
        &mut self,
        destination: &Destination,
        prepared: Prepared,
    ) -> Result<Receipt> {
        use crate::desktop::session_binding::Bound;
        if !destination.matches(self)
            || self
                .workspace
                .units()
                .iter()
                .any(|unit| unit.as_str() == prepared.binding.unit())
        {
            return Err(stale());
        }
        let cancel = AtomicBool::new(false);
        let catalog = self.catalog.borrow().loaded.clone();
        let states = Bound::from_prepared(prepared.restore, catalog.clone())
            .into_parts_for(catalog.as_ref(), &cancel)
            .map_err(|_| stale())?
            .prepare_states(&cancel)?;
        let windows = self.prepare_restored_batch(states, &cancel, |placement| {
            destination
                .space
                .physical(placement.rect)
                .map_err(|_| stale())
        })?;
        if !destination.matches(self) {
            return Err(stale());
        }
        let append = self.prepare_append(
            windows,
            destination.source.clone(),
            prepared.retained,
            destination.space,
        )?;
        // Take over while hidden first; allow receiver consumers to display only after a real checkpoint confirms it, never equating request success with display success.
        self.publish_append(append, false)?;
        Ok(Receipt {
            binding: prepared.binding,
            workspace: self.workspace.identity_stamp(),
            source: self.restored_session.as_ref().unwrap().original.clone(),
            consent: destination.consent.clone(),
            participation: destination.participation.clone(),
        })
    }
}
fn stale() -> Error {
    Error::new(
        E_ABORT,
        "Account projection destination changed or unavailable",
    )
}
