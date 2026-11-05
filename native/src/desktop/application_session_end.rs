//! 系统注销/关机只消费现有持久化凭证；不授新保存路径、丢弃批准或外部动作。
use super::*;
impl Application {
    pub(super) unsafe fn session_end_ready(&self) -> bool {
        if !restore_batch::guard::is_idle(self)
            || self.restore_job.is_some()
            || self.save_job.is_some()
            || self.live_freeze_job.is_some()
            || self.exit_after_save
            || self.open_after_save.is_some()
            || self.naming_import_job.is_some()
            || self.naming_save_job.is_some()
            || self.naming_autosave_blocks_exit()
            || self.automatic_save_pending()
            || self.checkpoint_exit_waiting()
            || !self.local_arts.session_end_idle()
            || self
                .live_sessions
                .iter()
                .any(|session| session.belongs_to(&self.workspace) && !session.exit_idle())
            || self.documents.iter().any(|doc| {
                doc.parameters
                    .as_ref()
                    .is_some_and(|panel| !panel.exit_idle())
                    || doc.state.save_session_requested.get()
                    || doc.state.save_existing_requested.get()
                    || doc.state.open_session_requested.get()
                    || doc.state.open_session_assets_requested.get()
                    || doc.state.naming_import_requested.get()
                    || doc.state.naming_save_requested.get()
                    || doc.state.naming_update_requested.get()
                    || doc.state.capture_protection_requested.get()
                    || doc.state.workflow_sync_requested.get().is_some()
                    || doc.state.live_request.borrow().is_some()
            })
            || !save_snapshot::automatic_ready(self).unwrap_or(false)
        {
            return false;
        }
        if self.save_status() == save_state::Status::Unchanged {
            return true;
        }
        self.session_checkpoint_current()
    }
}
