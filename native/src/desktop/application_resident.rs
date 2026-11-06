//! 用户删空工作区后保留不可见命令宿主；复用热键/托盘与现有 transient_host 保存语义。
use super::*;

impl Application {
    pub(super) unsafe fn resume_empty_workspace(&mut self, hotkeys_enabled: bool) -> Result<()> {
        self.resume_empty_with(|doc| {
            if hotkeys_enabled {
                super::super::global_capture::install(doc.window.0, &doc.state);
            }
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
        // 先落盘真实空工作区，不能让保活用的空白文档变成一张已保存的钉图。
        self.finish_deleted_workspace()?;
        let state = State::new(None, false)?;
        let unit = state.unit_id.clone();
        self.add_state(state, false)?;
        if let Some(record) = self.restored_session.as_mut() {
            record.transient_host = Some(unit);
        }
        let doc = &self.documents[0];
        if let Err(error) = install(doc) {
            eprintln!("native-resident-start: {error}");
        }
        Ok(())
    }
}
