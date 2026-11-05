//! 应用级回收偏好编辑；模态草稿不改来源基线，确认后触发裁剪，持久化仍走配置保存。
use super::*;
use crate::session_library::recycle::policy::Policy;
#[path = "recycle_dialog.rs"]
mod dialog;
#[path = "application_recycle_settings_smoke.rs"]
pub(in crate::desktop::application) mod smoke;
impl Application {
    pub(in crate::desktop::application) unsafe fn process_recycle_settings(
        &mut self,
        visible: bool,
    ) -> Result<()> {
        let result = self.edit_recycle_settings_with(|_, hwnd, old| dialog::show(hwnd, old));
        self.reference_result(result, visible)
    }
    unsafe fn edit_recycle_settings_with(
        &mut self,
        choose: impl FnOnce(&Self, HWND, Policy) -> Option<Policy>,
    ) -> Result<()> {
        if self.modals.blocked() {
            return Ok(());
        }
        let request = self.frozen_library.borrow_mut().configure_recycle.take();
        let Some(unit) = request else {
            return Ok(());
        };
        if self.save_job.is_some() || self.restore_job.is_some() || self.exit_requested.get() {
            return Err(Error::new(
                E_ABORT,
                "Workspace is busy during recycle settings request",
            ));
        }
        let doc = self
            .documents
            .iter()
            .find(|doc| doc.state.unit_id == unit && doc.is_alive())
            .ok_or_else(|| Error::new(E_ABORT, "Recycle settings requester closed"))?;
        let hwnd = doc.window.0;
        let flag = doc.state.file_dialog.clone();
        let before = self.recycle_policy;
        let source = self.naming_document.clone();
        let guard = crate::desktop::file_dialog::Guard::begin(&flag)
            .ok_or_else(|| Error::new(E_ABORT, "Another dialog is active"))?;
        let next = choose(self, hwnd, before);
        drop(guard);
        let Some(next) = next else {
            return Ok(());
        };
        next.validate()
            .map_err(|error| Error::new(E_INVALIDARG, error))?;
        if self.source_window(&unit) != Some(hwnd)
            || self.recycle_policy != before
            || !std::sync::Arc::ptr_eq(&source, &self.naming_document)
        {
            return Err(Error::new(
                E_ABORT,
                "Recycle preferences changed during dialog",
            ));
        }
        if next != before {
            self.recycle_policy = next;
            self.schedule_recycle_maintenance();
        }
        Ok(())
    }
}
