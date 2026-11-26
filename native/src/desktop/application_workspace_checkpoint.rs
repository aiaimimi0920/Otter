//! Continuous default-workspace checkpoints: share the existing save slot/Receipt, advancing only an independent disk baseline without marking the file saved.
use super::*;
use crate::{desktop::workspace_profile::Profile, session_source::Source};
use std::sync::atomic::AtomicBool;
#[path = "application_workspace_checkpoint_exit.rs"]
mod final_exit;
#[path = "application_projection_checkpoint_proof.rs"]
pub(super) mod projection_proof;
#[path = "application_workspace_scope.rs"]
mod scope;
#[path = "application_workspace_checkpoint_startup.rs"]
mod startup;
#[cfg(test)]
#[path = "application_workspace_checkpoint_tests.rs"]
mod tests;

pub(super) struct Owner {
    profile: Profile,
    original: Option<Source>,
    revision: u64,
    stamp: Option<(save_state::Stamp, Source)>,
    projections: Option<crate::projection_delivery::binding::checkpoint::Index>,
    senders: Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    accounts: Option<crate::projection_account::scene::Index>,
    projection_workspace: Option<crate::workspace::IdentityStamp>,
    projection_scope: Option<crate::session_projection_scope::Proof>,
    projection_display: projection_proof::Documents,
    has_recycle_bin: bool,
    pending: bool,
    stopped: bool,
    exiting: bool,
    exit: Option<final_exit::Pending>,
}
impl Owner {
    pub(in crate::desktop::application) fn profile_root(&self) -> &std::path::Path {
        self.profile.path.parent().unwrap()
    }
    pub(super) fn projection_deleted_binding(
        &self,
        workspace: &crate::workspace::Workspace,
        unit: &str,
    ) -> Option<&crate::projection_delivery::binding::Binding> {
        // Grant only revocation eligibility: ordinary edits preserve workspace identity, while a full swap cannot use an old checkpoint to delete new associations.
        if !self.projection_workspace.as_ref()?.matches(workspace) {
            return None;
        }
        self.projections.as_ref()?.for_unit(unit)
    }
    pub(super) fn projection_baseline(
        &self,
    ) -> Option<(
        &crate::projection_delivery::binding::checkpoint::Index,
        &Source,
    )> {
        Some((self.projections.as_ref()?, &self.stamp.as_ref()?.1))
    }
    pub(super) fn new(profile: Profile) -> Self {
        Self {
            profile,
            original: None,
            revision: 0,
            stamp: None,
            projections: None,
            senders: None,
            accounts: None,
            projection_workspace: None,
            projection_scope: None,
            projection_display: None,
            has_recycle_bin: false,
            pending: false,
            stopped: false,
            exiting: false,
            exit: None,
        }
    }
}
impl Application {
    /// Only for read-only system-exit admission; requires the complete current checkpoint actually committed, never an in-flight Receipt.
    pub(super) unsafe fn session_checkpoint_current(&self) -> bool {
        self.documents
            .iter()
            .all(|document| document.state.inline_text.borrow().is_none())
            && self.workspace_checkpoint.as_ref().is_some_and(|owner| {
                !owner.stopped
                    && !owner.pending
                    && !owner.exiting
                    && owner.exit.is_none()
                    && owner.stamp.as_ref().is_some_and(|(stamp, source)| {
                        stamp.matches_automatic(self)
                            && self.restored_session.as_ref().is_some_and(|record| {
                                source.same_storage(&record.original)
                                    && owner.projection_scope.as_ref().is_some_and(|proof| {
                                        // stamp's source is the retained input; Proof binds the actually persisted candidate, so these identities must not be mixed.
                                        owner
                                            .original
                                            .as_ref()
                                            .is_some_and(|published| proof.is_for(published))
                                            && record.projection_scope == Some(proof.scope())
                                    })
                            })
                    })
            })
    }
    pub(super) fn checkpoint_is_exiting(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| owner.exiting)
    }
    pub(super) fn projection_checkpoint_available(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| !owner.stopped && !owner.exiting && owner.exit.is_none())
    }
    pub(super) fn checkpoint_pending(&self) -> bool {
        self.workspace_checkpoint
            .as_ref()
            .is_some_and(|owner| owner.pending)
    }
    pub(super) fn checkpoint_exiting(&mut self) {
        if let Some(owner) = &mut self.workspace_checkpoint {
            owner.exiting = true;
        }
    }
    pub(super) unsafe fn pending_checkpoint_with(
        &mut self,
        notify: &mut impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if let Err(error) = self.poll_checkpoint() {
            self.stop_checkpoint(error, notify);
        }
    }
    pub(super) unsafe fn start_checkpoint_with(
        &mut self,
        notify: &mut impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if let Err(error) = self.start_checkpoint() {
            self.stop_checkpoint(error, notify);
        }
    }
    unsafe fn stop_checkpoint(
        &mut self,
        error: Error,
        notify: &mut impl FnMut(&Self, &crate::workspace_id::UnitId, &str, bool),
    ) {
        if let Some(owner) = &mut self.workspace_checkpoint {
            owner.stopped = true;
            owner.exit = None;
        }
        let message = crate::ui_text::format(
            "app-workspace-checkpoint-stopped",
            &[("error", error.to_string().into())],
        );
        eprintln!("native-workspace-checkpoint-stopped: {error}");
        if let Some(doc) = self.documents.iter().find(|doc| doc.is_alive()) {
            notify(self, &doc.state.unit_id, &message, true);
        }
    }
    unsafe fn start_checkpoint(&mut self) -> Result<()> {
        self.start_checkpoint_mode(false).map(|_| ())
    }
    unsafe fn start_checkpoint_mode(&mut self, force: bool) -> Result<bool> {
        let Some(owner) = &self.workspace_checkpoint else {
            return Ok(false);
        };
        if owner.stopped
            || owner.exiting
            || owner.exit.is_some()
            || self.save_job.is_some()
            || self.restore_job.is_some()
            || self.exit_after_save
            || self.open_after_save.is_some()
            || !save_snapshot::automatic_static_ready(self)
            || !restore_batch::guard::is_idle(self)
            || self
                .documents
                .iter()
                .any(|doc| doc.state.open_session_requested.get())
        {
            return Ok(false);
        }
        if !force
            && owner.projection_scope.is_some()
            && owner.stamp.as_ref().is_some_and(|(stamp, source)| {
                stamp.matches_automatic(self)
                    && self
                        .restored_session
                        .as_ref()
                        .is_some_and(|record| source.same_storage(&record.original))
            })
        {
            self.reconcile_checkpoint_source();
            return Ok(false);
        }
        let destination = match &owner.original {
            Some(original) => {
                save_disk::Destination::Replace(owner.profile.path.clone(), original.clone())
            }
            None => save_disk::Destination::New(owner.profile.path.clone()),
        };
        // Advance the validated revision with the disk baseline; do not reparse the entire checkpoint on UI hot paths.
        let revision = owner.revision;
        let Some((receipt, data)) = self.capture_automatic_session_snapshot()? else {
            return Ok(false);
        };
        let job = save_snapshot::job::Job::start_checkpoint(
            data,
            destination,
            revision,
            self.execution_wake.callback(),
        )
        .map_err(|error| Error::new(E_FAIL, format!("Cannot start checkpoint: {error}")))?;
        self.save_job = Some((receipt, job));
        self.workspace_checkpoint.as_mut().unwrap().pending = true;
        Ok(true)
    }
    unsafe fn poll_checkpoint(&mut self) -> Result<()> {
        if !self.checkpoint_pending() {
            return Ok(());
        }
        let cancelled = self.exit_requested.get();
        if cancelled {
            self.cancel_save_candidate();
        }
        let result = self.poll_checkpoint_candidate();
        if matches!(result, Ok(save_control::Poll::Pending)) {
            return Ok(());
        }
        self.workspace_checkpoint.as_mut().unwrap().pending = false;
        // Live freeze/recycle and other intents retained during in-flight work must resume themselves, without waiting for incidental input.
        PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0))?;
        if cancelled {
            return Ok(()); // Exiting normally makes Receipt stale; do not tamper with the exit flag to bypass validation.
        }
        if let save_control::Poll::Ready(mut prepared) = result? {
            if !self.checkpoint_exit_matches() {
                return Ok(()); // Neither the final snapshot nor next-turn window closure may overtake new explicit intent.
            }
            let disk = prepared
                .staged
                .take()
                .ok_or_else(|| Error::new(E_FAIL, "Missing checkpoint file"))?;
            let candidate_source = Source::from(prepared.candidate.bytes.clone());
            let stale = || Error::new(E_ABORT, "Checkpoint changed before commit");
            if !disk.is_for(&prepared.candidate)
                || prepared
                    .checkpoint_senders
                    .as_ref()
                    .is_some_and(|index| !index.is_for(&candidate_source))
                || prepared.checkpoint_scope.as_ref().is_none_or(|proof| {
                    !proof.is_for(&candidate_source)
                        || self
                            .restored_session
                            .as_ref()
                            .and_then(|record| record.projection_scope)
                            != Some(proof.scope())
                })
                || prepared
                    .checkpoint_projections
                    .as_ref()
                    .is_some_and(|index| !index.is_for(&candidate_source))
                || prepared
                    .checkpoint_accounts
                    .as_ref()
                    .is_none_or(|index| !index.is_for(&candidate_source))
            {
                return Err(stale());
            }
            if !save_snapshot::automatic_ready(self)? {
                return Ok(()); // New Live gestures/explicit intent normally invalidate the background time point.
            }
            let stamp = prepared
                .receipt
                .checked_saved_stamp(self)
                .ok_or_else(stale)?;
            let display = projection_proof::capture(self, prepared.checkpoint_projections.as_ref());
            let source = self.restored_session.as_ref().unwrap().original.clone();
            // Do not pump UI messages; leave the profile baseline unchanged until success, then leave manual files/dirty/workflow untouched.
            let published = disk.commit(&AtomicBool::new(false)).map_err(|failure| {
                Error::new(E_FAIL, format!("Cannot commit checkpoint: {failure:?}"))
            })?;
            self.projection_receive
                .committed(prepared.checkpoint_projections.as_ref(), &self.workspace);
            self.projection_edit.committed(
                prepared.checkpoint_scope.as_ref(),
                &candidate_source,
                &self.workspace,
            );
            self.projection_sender
                .committed(prepared.checkpoint_senders.as_ref(), &self.workspace);
            self.projection_sender.stop_committed(
                prepared.checkpoint_senders.as_ref(),
                &candidate_source,
                &source,
                &self.workspace,
            );
            // Share the canonical buffer across checkpoint and default-session library state when recycle-bin JSON is involved and no manual/in-flight owner pins the previous Source.
            let adopt = (self.workspace_checkpoint.as_ref().unwrap().has_recycle_bin
                || prepared.has_recycle_bin)
                && self.can_adopt_checkpoint_source();
            let stamp_source = if adopt {
                self.restored_session.as_mut().unwrap().original = candidate_source.clone();
                candidate_source.clone()
            } else {
                source
            };
            let owner = self.workspace_checkpoint.as_mut().unwrap();
            owner.revision = prepared.candidate.revision;
            owner.original = Some(candidate_source);
            owner.stamp = Some((stamp, stamp_source));
            owner.has_recycle_bin = prepared.has_recycle_bin;
            // Record actual publication first; a separate failure to retire an old recovery copy must not erase the committed inclusion relationship.
            owner.projections = prepared.checkpoint_projections;
            owner.senders = prepared.checkpoint_senders;
            owner.accounts = prepared.checkpoint_accounts;
            owner.projection_scope = prepared.checkpoint_scope;
            owner.projection_workspace = Some(self.workspace.identity_stamp());
            owner.projection_display = display;
            if adopt {
                self.sync_frozen_menu();
            }
            if let Ok(mut view) = self.authorization.try_borrow_mut() {
                view.sender.stop_source = self
                    .projection_sender_scope()
                    .and_then(|_| self.workspace_checkpoint.as_ref()?.sender_source().cloned());
                view.sender.bindings = self
                    .workspace_checkpoint
                    .as_ref()
                    .and_then(|owner| owner.sender_index())
                    .map(|index| index.entries().to_vec())
                    .unwrap_or_default();
            }
            if let Some((path, error)) = published.retained_recovery {
                return Err(Error::new(
                    E_FAIL,
                    crate::ui_text::format(
                        "app-checkpoint-recovery-retained",
                        &[
                            ("path", path.display().to_string().into()),
                            ("error", error.to_string().into()),
                        ],
                    ),
                ));
            }
        }
        Ok(())
    }
    /// WM_QUIT is queued after the last Unit is deleted; finish the empty workspace before Wake closes, without recording application exit itself as empty.
    pub(super) unsafe fn finish_deleted_workspace(&mut self) -> Result<()> {
        if self
            .workspace_checkpoint
            .as_ref()
            .is_none_or(|owner| owner.stopped || owner.exiting)
        {
            return Ok(());
        }
        self.collect_closed();
        if !self.documents.is_empty() {
            return Ok(());
        }
        // The final source is destroyed, so manual/library/restore candidates can no longer publish; cancel and reap through the original Job Drop.
        self.remote_sessions.finish_projection();
        self.collect_projection_abandoned();
        self.flush_projection_sender()?;
        self.projection_receive
            .shutdown(&mut self.projection_cleanup);
        self.projection_profile.finish_open();
        self.projection_profile.refresh(); // A direct terminal join must also synchronize newly created intent instead of pruning it using old entries.
        self.projection_cleanup.resolve(&self.projection_profile);
        self.projection_cleanup.flush().map_err(|error| {
            Error::new(
                E_FAIL,
                format!("Cannot persist projection cleanup: {error:?}"),
            )
        })?;
        // This is application terminal state only; never wait synchronously for encoding in an individual HWND callback or while documents remain.
        self.cancel_restore();
        self.restore_job.take();
        self.save_origin = None;
        self.open_after_save = None;
        self.exit_after_save = false;
        self.recycle_maintenance_pending = false;
        if !self.checkpoint_pending() {
            self.save_job.take();
        }
        loop {
            self.pending_windows(false)?; // Retire the final source's input/execution tasks; completion still proceeds through the original Wake.
            self.poll_checkpoint()?;
            self.start_checkpoint()?;
            if !self.checkpoint_pending() && restore_batch::guard::is_idle(self) {
                let owner = self.workspace_checkpoint.as_ref().unwrap();
                return if owner.stamp.as_ref().is_some_and(|(stamp, source)| {
                    stamp.matches_automatic(self)
                        && self
                            .restored_session
                            .as_ref()
                            .is_some_and(|record| source.same_storage(&record.original))
                }) {
                    Ok(())
                } else {
                    Err(Error::new(
                        E_ABORT,
                        "Empty workspace checkpoint is blocked by pending work",
                    ))
                };
            }
            let mut message = MSG::default();
            let deadline = self
                .save_reap_deadline()
                .into_iter()
                .chain(self.workflow_reap_deadline())
                .chain(self.account_projection.reap_deadline())
                .min();
            if live::wait::next_until(&[], &mut message, deadline)? == 0 {
                return Err(Error::new(
                    E_ABORT,
                    "Message loop ended before empty checkpoint",
                ));
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    /// Default-profile sessions have no external manual-file revision; adopt the committed checkpoint buffer only when no in-flight operation checks same_storage against the old Source.
    fn can_adopt_checkpoint_source(&self) -> bool {
        let Some(record) = &self.restored_session else {
            return false;
        };
        if record.saved_path.is_some()
            || record.recovery_path.is_some()
            || record.disk_original.is_some()
            || !record.pending_arts.is_empty()
            || record.workflow.is_some()
            || record.workflow_sync.is_some()
            || record.workflow_delivery.is_some()
            || self.live_freeze_job.is_some()
            || self.checkpoint_exit_waiting()
            || self.exit_saved.is_some()
            || self.account_projection.busy()
            || self.projection_receive.busy()
            || self.projection_sender.busy()
            || self.projection_edit.busy()
        {
            return false;
        }
        let frozen = self.frozen_library.borrow();
        frozen.menu.is_none()
            && frozen.request.is_none()
            && frozen.reference.is_none()
            && frozen.configure_recycle.is_none()
    }
    fn reconcile_checkpoint_source(&mut self) {
        if !self.can_adopt_checkpoint_source() {
            return;
        }
        let Some(canonical) = self.workspace_checkpoint.as_ref().and_then(|owner| {
            let published = owner.original.as_ref()?;
            (owner.has_recycle_bin
                && owner
                    .projection_scope
                    .as_ref()
                    .is_some_and(|proof| proof.is_for(published))
                && self
                    .restored_session
                    .as_ref()
                    .is_some_and(|record| !published.same_storage(&record.original)))
            .then(|| published.clone())
        }) else {
            return;
        };
        self.restored_session.as_mut().unwrap().original = canonical.clone();
        self.workspace_checkpoint
            .as_mut()
            .unwrap()
            .stamp
            .as_mut()
            .unwrap()
            .1 = canonical;
        self.sync_frozen_menu();
    }
}
