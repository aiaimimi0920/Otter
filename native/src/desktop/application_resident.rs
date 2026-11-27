//! Retain an invisible command host after the user empties the workspace; reuse hotkeys/tray and existing transient_host save semantics.
use super::*;

impl Application {
    pub(super) unsafe fn resume_empty_workspace(&mut self, hotkeys_enabled: bool) -> Result<()> {
        self.resume_empty_with(|doc| {
            doc.state.hotkey_enabled.set(hotkeys_enabled);
            super::super::global_capture::install(doc.window.0, &doc.state);
            super::super::tray::start(doc.window.0, &doc.state)
        })
    }
    pub(super) unsafe fn resume_empty_with(
        &mut self,
        install: impl FnOnce(&DocumentWindow) -> Result<()>,
    ) -> Result<()> {
        self.collect_closed();
        if !self.documents.is_empty() || self.exit_committed || self.session_end.ended() {
            return Ok(());
        }
        // Persist the genuinely empty workspace first; the keepalive blank document must not become a saved pin.
        self.finish_deleted_workspace()?;
        let state = State::new(None, false)?;
        let unit = state.unit_id.clone();
        self.add_state(state, false)?;
        if let Some(record) = self.restored_session.as_mut() {
            record.transient_host = Some(unit);
        }
        self.ensure_tray_with(install);
        Ok(())
    }
}
