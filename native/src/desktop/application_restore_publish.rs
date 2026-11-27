//! Native session publication: all recoverable failures precede the owner swap; write no files and preserve unknown session fields.
use super::*;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_restore_publish_smoke.rs"]
pub(super) mod smoke;

pub(in crate::desktop::application) struct Record {
    pub restored_accounts: Option<crate::projection_account::scene::Index>,
    pub restored_senders: Option<crate::projection_delivery::sender::binding::checkpoint::Index>,
    pub original: crate::session_source::Source,
    /// Supplied by the same background candidate, solely for the initial post-restore checkpoint baseline.
    pub restored_revision: u64,
    /// Read-only associations parsed by the original background candidate; consumed by the default-profile startup barrier, not proof that an ordinary import is persisted.
    pub restored_projections: Option<crate::projection_delivery::binding::checkpoint::Index>,
    pub restored_projection_scope: Option<crate::session_projection_scope::Proof>,
    /// Allocated only once in memory; no send eligibility is granted without proof of inclusion in the canonical checkpoint.
    pub projection_scope: Option<crate::session_projection_scope::Scope>,
    /// Library edits may change in-memory source content; replacing an existing file still requires comparing the complete bytes of the last actual commit.
    pub disk_original: Option<crate::session_source::Source>,
    pub saved_path: Option<std::path::PathBuf>,
    pub recovery_path: Option<std::path::PathBuf>,
    pub space: Option<crate::session_placement::Space>,
    pub annotation_ids: Vec<(crate::workspace_id::UnitId, Vec<String>)>,
    /// Initial runtime entry for an empty session; later writes must inspect its current content, not permanently exclude user edits based on this ID alone.
    pub transient_host: Option<crate::workspace_id::UnitId>,
    pub pending_arts: Vec<crate::workspace_id::UnitId>,
    pub binding_catalog: Option<Rc<Loaded>>,
    pub attempted_arts: Vec<crate::workspace_id::UnitId>,
    pub workflow: Option<super::workflow::Pending>,
    pub workflow_cycle: Option<super::workflow::cycle::Cycle>,
    pub workflow_settings: Option<super::workflow::ui::Settings>,
    pub workflow_sync: Option<super::workflow::sync::Sync>,
    pub workflow_delivery: Option<Result<super::workflow::sync::completion::Completion>>,
}

impl Record {
    pub(in crate::desktop::application) fn edit_original(
        &mut self,
        source: crate::session_source::Source,
    ) {
        if self.saved_path.is_some() && self.disk_original.is_none() {
            self.disk_original = Some(self.original.clone());
        }
        self.original = source;
        self.restored_projections = None;
        self.restored_senders = None;
        self.restored_accounts = None;
        self.restored_projection_scope = None;
    }
}

impl Application {
    /// The caller must run after the UI message callback returns and already hold user intent to replace the current workspace.
    /// visible=false supports hidden-HWND integration verification; release old window owners only after success.
    pub unsafe fn publish_restored_batch(
        &mut self,
        mut batch: Windows,
        visible: bool,
    ) -> Result<()> {
        if !batch.matches_application(self) {
            return Err(Error::new(E_ABORT, "Restored workspace changed or busy"));
        }
        let transient_host = self.prepare_empty_restore_host(&mut batch)?;
        let groups = batch.candidate.workspace.groups().to_vec();
        let mut capture_windows = Vec::new();
        capture_windows
            .try_reserve_exact(batch.documents.len() * 2)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot prepare restored capture registry"))?;
        for doc in &batch.documents {
            capture_windows.extend([doc.window.0, doc.panel.0]);
            if visible {
                doc.observe_priority()?;
            }
        }
        // Revalidate after installing listeners. Until the swap, only transfer hotkeys; do not dispatch UI messages or execute nodes.
        if !batch.matches_application(self) {
            return Err(Error::new(
                E_ABORT,
                "Workspace changed before restoration publication",
            ));
        }
        transfer_hotkey(&self.documents, &batch.documents, |doc, enabled| {
            crate::desktop::shortcut_registry::transfer(doc.window.0, &doc.state, enabled)
        })?;
        let workspace = std::mem::take(&mut batch.candidate.workspace);
        let (saved_path, recovery_path) = match batch.candidate.origin.take() {
            Some(super::super::super::session_candidate::Origin {
                requested,
                recovery: None,
            }) => (Some(requested), None),
            Some(super::super::super::session_candidate::Origin {
                recovery: Some(path),
                ..
            }) => (None, Some(path)),
            None => (None, None),
        };
        let record = Record {
            restored_senders: batch.candidate.sender_index.take(),
            restored_accounts: batch.candidate.account_index.take(),
            projection_scope: batch
                .candidate
                .projection_scope
                .as_ref()
                .map(|proof| proof.scope()),
            restored_projection_scope: batch.candidate.projection_scope.take(),
            restored_projections: batch.candidate.projection_index.take(),
            disk_original: None,
            workflow_cycle: None,
            workflow_settings: None,
            workflow_sync: None,
            workflow_delivery: None,
            workflow: None,
            pending_arts: batch
                .documents
                .iter()
                .filter(|doc| doc.state.restored_art && doc.state.selected_art.borrow().is_none())
                .map(|doc| doc.state.unit_id.clone())
                .collect(),
            binding_catalog: batch.catalog.clone(),
            attempted_arts: Vec::new(),
            restored_revision: batch.candidate.revision,
            original: batch.candidate.into_original_json(),
            saved_path,
            recovery_path,
            space: None,
            annotation_ids: batch.annotation_ids,
            transient_host,
        };
        // The following is a synchronous ownership swap: Box<State> addresses in candidate HWNDs stay unchanged; perform no further fallible preparation.
        self.stop_projection_sender();
        // Closing old Art HWNDs may reenter; revoke authority while the complete old workspace remains installed, never midway through the ownership swap.
        self.local_arts.reset();
        self.reset_pin_layout();
        let previous = std::mem::replace(&mut self.documents, batch.documents);
        self.workspace = workspace;
        self.workspace_autosave.disable();
        self.reset_projection_inbox();
        if let Some(mut previous) = self.restored_session.replace(record) {
            self.retire_workspace_workflow(&mut previous);
        }
        self.executions.clear();
        self.display_cache.borrow_mut().take();
        self.display_composition.borrow_mut().invalidate();
        *self.group_catalog.borrow_mut() = groups;
        *self.capture_windows.borrow_mut() = capture_windows;
        // manifest already validated candidate State group membership/lock/visibility; do not trigger group commands during the swap.
        if visible {
            for doc in &self.documents {
                doc.show_content(SW_SHOWNOACTIVATE);
            }
        }
        drop(previous);
        // coordinator has not installed coordinate context yet; do not reuse the old workspace's save credentials.
        self.saved_state = None;
        if self
            .restored_session
            .as_ref()
            .is_some_and(|record| record.recovery_path.is_some())
        {
            println!("native-session-recovery: loaded=true save-as-required=true");
        }
        if visible {
            if let Some(path) = self
                .restored_session
                .as_ref()
                .and_then(|record| record.recovery_path.as_ref())
            {
                if let Some(document) = self.documents.first() {
                    windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
                        Some(document.window.0),
                        &windows::core::HSTRING::from(crate::ui_text::format(
                            "app-session-recovered",
                            &[("path", path.display().to_string().into())],
                        )),
                        &windows::core::HSTRING::from(crate::ui_text::text(
                            "app-session-recovery-title",
                        )),
                        windows::Win32::UI::WindowsAndMessaging::MB_OK
                            | windows::Win32::UI::WindowsAndMessaging::MB_ICONWARNING,
                    );
                }
            }
        }
        Ok(())
    }

    unsafe fn prepare_empty_restore_host(
        &self,
        batch: &mut Windows,
    ) -> Result<Option<crate::workspace_id::UnitId>> {
        if !batch.documents.is_empty() {
            return Ok(None);
        }
        batch
            .documents
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve empty workspace host"))?;
        batch
            .ready
            .try_reserve(1)
            .map_err(|_| Error::new(E_OUTOFMEMORY, "Cannot reserve empty workspace guard"))?;
        let document = self.prepare_window(State::new(None, false)?, false)?;
        let unit = document.state.unit_id.clone();
        batch
            .candidate
            .workspace
            .add_units([unit.clone()])
            .map_err(|error| {
                Error::new(
                    E_FAIL,
                    format!("Cannot register empty workspace host: {error:?}"),
                )
            })?;
        batch.ready.push(guard::Document::capture(&document)?);
        batch.documents.push(document);
        Ok(Some(unit))
    }
}

pub(in crate::desktop::application) fn transfer_hotkey(
    previous: &[DocumentWindow],
    next: &[DocumentWindow],
    mut change: impl FnMut(&DocumentWindow, bool) -> Result<()>,
) -> Result<()> {
    let Some(owner) = previous
        .iter()
        .find(|doc| crate::desktop::shortcut_registry::owns(&doc.state))
    else {
        return Ok(());
    };
    let target = next
        .first()
        .ok_or_else(|| Error::new(E_ABORT, "Missing restored hotkey owner"))?;
    let preferred = owner.state.hotkey_enabled.get();
    change(owner, false)?;
    if let Err(error) = change(target, true) {
        let rollback = change(owner, true);
        owner.state.hotkey_enabled.set(preferred);
        return match rollback {
            Ok(()) => Err(error),
            Err(rollback) => Err(Error::new(E_FAIL, format!(
                "Restoration hotkey transfer failed: {error}; original hotkey recovery failed: {rollback}"
            ))),
        };
    }
    target.state.hotkey_enabled.set(preferred);
    Ok(())
}
