//! 用户显式开启的事件驱动配置保存；单槽、无空闲计时器，错误后停用而非持续重试。
use super::*;
use std::path::PathBuf;

impl Application {
    pub(super) unsafe fn pending_naming_save_auto_with(
        &mut self,
        choose: impl FnOnce(HWND, &State) -> Result<Option<PathBuf>>,
    ) -> Result<Option<String>> {
        let enabled = self.naming_autosave.get();
        let waiting_exit = self.exit_requested.get() && self.naming_autosave_blocks_exit();
        let result = self
            .schedule_naming_autosave()
            .and_then(|()| self.pending_naming_save_with(choose));
        if enabled && result.is_err() {
            self.naming_autosave.set(false);
            self.naming_autosave_request = false;
            self.exit_requested.set(false);
        } else if waiting_exit && !self.naming_autosave_blocks_exit() {
            // 应用退出曾等待配置保存；完成后重新进入退出确认，不在保存栈里销毁窗口。
            PostMessageW(None, WM_NULL, WPARAM(0), LPARAM(0))?;
        }
        result.map_err(|error| {
            if enabled {
                Error::new(
                    error.code(),
                    format!("自动保存已停止；当前设置仍保留，请解决错误后重新开启。\n{error}"),
                )
            } else {
                error
            }
        })
    }
    unsafe fn schedule_naming_autosave(&mut self) -> Result<()> {
        if !self.naming_autosave.get()
            || self.modals.blocked()
            || self.naming_save_job.is_some()
            || self.naming_import_job.is_some()
        {
            return Ok(());
        }
        // 用户的显式另存为/保存优先，不能把它们改标为自动任务。
        if self.documents.iter().any(|doc| {
            doc.state.naming_save_requested.get() || doc.state.naming_update_requested.get()
        }) {
            return Ok(());
        }
        let source = self.naming_source.as_ref();
        if source.is_none() && self.naming_profile_target.is_none() {
            return Err(Error::new(
                E_INVALIDARG,
                "请先加载配置或将设置另存为，再开启自动保存。",
            ));
        }
        if source.is_some_and(|source| source.recovery_target().is_some()) {
            return Err(Error::new(
                E_INVALIDARG,
                "恢复来源必须先另存为，不能开启自动覆盖保存。",
            ));
        }
        if source
            .is_some_and(|source| !std::sync::Arc::ptr_eq(source.document(), &self.naming_document))
        {
            return Err(Error::new(
                E_ABORT,
                "Settings source no longer matches document",
            ));
        }
        if !self.preferences_changed() {
            return Ok(());
        }
        if let Some(doc) = self.documents.iter().find(|doc| doc.is_alive()) {
            doc.state.naming_update_requested.set(true);
            self.naming_autosave_request = true;
        }
        Ok(())
    }
    pub(super) fn naming_autosave_blocks_exit(&self) -> bool {
        self.naming_autosave.get()
            && (self.naming_save_job.is_some()
                || self.preferences_changed()
                || (self.naming_source.is_none() && self.naming_profile_target.is_none()))
    }
    fn preferences_changed(&self) -> bool {
        *self.naming.borrow() != *self.naming_document.naming()
            || self.naming_document.cache_policy().ok() != Some(self.cache_policy.get())
            || self.naming_document.recycle_policy().ok() != Some(self.recycle_policy)
    }
}
