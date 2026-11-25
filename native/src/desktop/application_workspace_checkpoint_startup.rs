//! Startup restore barrier: accept global capture or enter the normal message loop only after file preparation and hidden-window creation finish.
use super::*;
impl Application {
    pub(in crate::desktop) unsafe fn install_workspace_checkpoint(
        &mut self,
        profile: Option<Profile>,
        check: bool,
    ) -> Result<()> {
        let Some(profile) = profile else {
            return Ok(());
        };
        let restore = profile.restore;
        let path = profile.path.clone();
        self.workspace_checkpoint = Some(Owner::new(profile));
        if restore {
            self.request_restore_input(crate::desktop::session_job::Input::File(path), None, None)?;
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
            let owner = self.workspace_checkpoint.as_mut().unwrap();
            owner.revision = record.restored_revision;
            owner.original = Some(record.original.clone());
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
            MessageBoxW(self.documents.first().map(|doc| doc.window.0),
                w!("默认工作区原文件缺失，已读取唯一恢复副本。副本保持原样，自动检查点已停用；请另存工作区。"),
                w!("Otter 工作区恢复"), MB_OK | MB_ICONWARNING);
        }
        if !check {
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
