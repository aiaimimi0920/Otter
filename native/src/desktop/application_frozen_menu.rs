//! 菜单请求在窗口回调返回后交付到已有单任务追加协调器；再次验证请求窗口和来源。
use super::*;
#[path = "application_recycle_maintenance.rs"]
pub(super) mod maintenance;
#[path = "application_recycle_close.rs"]
pub(super) mod recycle_close;
#[path = "application_reference.rs"]
pub(super) mod reference;
#[path = "application_library_remove_smoke.rs"]
pub(super) mod remove_smoke;
#[path = "application_recycle_settings.rs"]
pub(super) mod settings;
#[path = "application_frozen_menu_smoke.rs"]
pub(super) mod smoke;
impl Application {
    pub(super) fn sync_frozen_menu(&self) {
        let mut view = self.frozen_library.borrow_mut();
        view.source = self
            .restored_session
            .as_ref()
            .filter(|record| record.space.is_some())
            .map(|record| record.original.clone());
        view.busy =
            self.restore_job.is_some() || self.save_job.is_some() || self.exit_requested.get();
    }
    pub(super) unsafe fn process_frozen_menu(&mut self, visible: bool) -> Result<()> {
        self.process_recycle_settings(visible)?;
        self.process_reference(visible)?;
        let request = self.frozen_library.borrow_mut().request.take();
        let Some(request) = request else {
            let result = self.start_recycle_maintenance();
            return self.reference_result(result, visible);
        };
        let Some(document) = self.documents.iter().find(|doc| {
            doc.state.unit_id == request.origin
                && doc.is_alive()
                && request.target.matches(&doc.state.canvas.borrow())
        }) else {
            return Ok(());
        };
        let hwnd = document.window.0;
        let result = if request.choice.remove {
            self.request_frozen_remove(request.choice)
        } else {
            self.request_frozen(
                request.choice.source,
                request.choice.kind,
                request.choice.entry_id,
                request.point,
            )
            .map(|_| ())
        };
        self.sync_frozen_menu();
        if let Err(error) = result {
            if !visible {
                return Err(error);
            }
            MessageBoxW(
                Some(hwnd),
                &HSTRING::from(format!("无法处理冻结贴纸，原工作区和原库保留。\n{error}")),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
        Ok(())
    }
}
