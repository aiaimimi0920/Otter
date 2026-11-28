//! Startup restore barrier: accept global capture or enter the normal message loop only after file preparation and hidden-window creation finish.
use super::*;
impl Application {
    /// Exercise profile workers against the fixture directory, not the runner's desktop profile.
    #[cfg(test)]
    pub(in crate::desktop) unsafe fn install_projection_fixture(
        &mut self,
        profile: Profile,
    ) -> Result<()> {
        let root = profile.path.parent().unwrap().to_owned();
        self.install_workspace_checkpoint(Some(profile), false)?;
        if !crate::elevation_status::allows_desktop_integrations() {
            assert!(matches!(
                self.projection_profile,
                crate::desktop::application::projection::profile::Owner::Disabled
            ));
            self.projection_sender
                .start(root.clone(), self.execution_wake.callback());
            self.projection_profile
                .start(root, self.execution_wake.callback());
        }
        Ok(())
    }
    pub(in crate::desktop) unsafe fn install_workspace_checkpoint(
        &mut self,
        profile: Option<Profile>,
        check: bool,
    ) -> Result<()> {
        let Some(mut profile) = profile else {
            return Ok(());
        };
        let restore = profile.restore;
        if profile.claimed.is_some() && !check {
            return Err(Error::new(
                E_ABORT,
                "A checkpoint check does not authorize desktop startup",
            ));
        }
        let (input, expected_revision) = match profile.claimed.as_mut() {
            Some(c) => (
                crate::desktop::session_job::Input::Bytes(std::mem::take(&mut c.bytes)),
                Some(c.revision),
            ),
            None => (
                crate::desktop::session_job::Input::File(profile.path.clone()),
                None,
            ),
        };
        self.workspace_checkpoint = Some(Owner::new(profile));
        if restore {
            self.request_restore_input(input, None, None)?;
            loop {
                self.execution_wake.acknowledge();
                // Errors abort default startup immediately; never treat a corrupt/unknown workspace as empty and overwrite it.
                self.process_restore(false)?;
                if self.restore_job.is_none() {
                    break;
                }
                let mut message = MSG::default();
                let deadline = self.restore_reap_deadline();
                if crate::desktop::application::live::wait::next_until(&[], &mut message, deadline)?
                    == 0
                {
                    return Err(Error::new(E_ABORT, "Workspace startup interrupted"));
                }
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            let record = self.restored_session.as_mut().expect("restored checkpoint");
            if expected_revision.is_some_and(|revision| {
                record.recovery_path.is_some() || record.restored_revision != revision
            }) {
                return Err(Error::new(
                    E_ABORT,
                    "Restored checkpoint revision differs from the published revision",
                ));
            }
            let owner = self.workspace_checkpoint.as_mut().unwrap();
            owner.revision = record.restored_revision;
            owner.original = Some(record.original.clone());
            owner.has_recycle_bin = crate::session_library::decode(
                &record.original,
                crate::session_library::Kind::RecycleBin,
            )
            .is_ok_and(|library| !library.entries().is_empty());
            owner.stopped = record.recovery_path.is_some();
            if !owner.stopped {
                owner.accounts = record
                    .restored_accounts
                    .take()
                    .filter(|index| index.is_for(&record.original));
                owner.senders = record
                    .restored_senders
                    .take()
                    .filter(|index| index.is_for(&record.original));
                owner.projection_scope = record
                    .restored_projection_scope
                    .take()
                    .filter(|proof| proof.is_for(&record.original));
                owner.projection_workspace = Some(self.workspace.identity_stamp());
            }
            if !owner.stopped
                && record
                    .restored_projections
                    .as_ref()
                    .is_some_and(|index| index.is_for(&record.original))
            {
                owner.projections = record.restored_projections.take();
                owner.projection_workspace = Some(self.workspace.identity_stamp());
            }
            // Default restoration is not a user-selected save file; it must not implicitly authorize Ctrl+S to write the profile.
            record.saved_path = None;
            record.disk_original = None;
            self.saved_state = None;
            let stamp = save_state::Stamp::capture(self)?;
            self.workspace_checkpoint.as_mut().unwrap().stamp = Some((
                stamp,
                self.restored_session.as_ref().unwrap().original.clone(),
            ));
            let display = projection_proof::capture(
                self,
                self.workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .projections
                    .as_ref(),
            );
            self.workspace_checkpoint
                .as_mut()
                .unwrap()
                .projection_display = display;
            self.sync_frozen_menu();
        }
        let readonly = self.workspace_checkpoint.as_ref().unwrap().stopped;
        if check {
            println!("native-workspace-startup: loaded={restore} readonly={readonly} manual-target=false");
            println!(
                "native-workspace-sender-scope: durable={}",
                self.projection_sender_scope().is_some()
            );
            println!(
                "native-workspace-sender-bindings: durable={}",
                self.workspace_checkpoint
                    .as_ref()
                    .and_then(|owner| owner.senders.as_ref())
                    .map(|index| index
                        .entries()
                        .iter()
                        .filter(|binding| self.projection_sender_inclusion(binding))
                        .count())
                    .unwrap_or(0)
            );
        } else if readonly {
            MessageBoxW(
                self.documents.first().map(|doc| doc.window.0),
                &windows::core::HSTRING::from(crate::ui_text::text(
                    "app-workspace-default-recovery",
                )),
                &windows::core::HSTRING::from(crate::ui_text::text("app-workspace-recovery-title")),
                MB_OK | MB_ICONWARNING,
            );
        }
        if !check && crate::elevation_status::allows_desktop_integrations() {
            self.projection_sender.start(
                self.workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .profile
                    .path
                    .parent()
                    .unwrap()
                    .to_owned(),
                self.execution_wake.callback(),
            );
            self.projection_profile.start(
                self.workspace_checkpoint
                    .as_ref()
                    .unwrap()
                    .profile
                    .path
                    .parent()
                    .unwrap()
                    .to_owned(),
                self.execution_wake.callback(),
            );
        }
        Ok(())
    }
}
