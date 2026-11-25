//! Deliver menu requests to the existing single-task append coordinator after window callbacks return; revalidate the requesting window and source.
use super::*;
#[path = "application_recycle_maintenance.rs"]
pub(super) mod maintenance;
#[path = "application_recycle_close.rs"]
pub(super) mod recycle_close;
#[path = "application_reference.rs"]
pub(super) mod reference;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_library_remove_smoke.rs"]
pub(super) mod remove_smoke;
#[path = "application_recycle_settings.rs"]
pub(super) mod settings;
#[cfg(any(test, debug_assertions, feature = "smoke-test"))]
#[path = "application_frozen_menu_smoke.rs"]
pub(super) mod smoke;
impl Application {
    pub(super) fn sync_frozen_menu(&self) {
        let mut view = self.frozen_library.borrow_mut();
        let source = self
            .restored_session
            .as_ref()
            .filter(|record| record.space.is_some())
            .map(|record| &record.original);
        let unchanged = match (&view.source, source) {
            (Some(current), Some(next)) => current.same_storage(next),
            (None, None) => true,
            _ => false,
        };
        if !unchanged {
            view.source = source.cloned();
        }
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
                &HSTRING::from(crate::ui_text::format(
                    "app-frozen-sticker-failed",
                    &[("error", error.to_string().into())],
                )),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
        Ok(())
    }
}
