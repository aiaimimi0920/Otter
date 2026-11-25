//! Application resource settlement: route Ready exactly and round-robin window requests so no scene monopolizes shared capacity.
use super::*;
use crate::surface_resource_runtime::{request::Request, Ready};
pub(crate) enum StartImageError {
    Busy,
    Failed(String),
}
impl Windows {
    pub(crate) fn install_ready(&mut self, ready: Ready, now: u64) {
        let Some(window) = self.entries.iter_mut().find(|w| {
            w.is_alive()
                && w.binding() == ready.binding()
                && w.snapshot_revision() == ready.revision()
        }) else {
            return;
        }; // Consume/release stale results instead of permanently occupying Ready-queue capacity.
        if let Err(error) = window.install_resource(ready, now) {
            window.record_image_error(&crate::ui_text::format(
                "integration-image-install-failed",
                &[("error", format!("{error:?}").into())],
            ));
        }
    }
    pub(crate) fn schedule_images(
        &mut self,
        now: u64,
        mut start: impl FnMut(Request) -> std::result::Result<(), StartImageError>,
    ) {
        if self.entries.is_empty() {
            return;
        }
        let first = self.image_cursor % self.entries.len();
        self.image_cursor = (first + 1) % self.entries.len();
        let (before, after) = self.entries.split_at_mut(first);
        for window in after.iter_mut().chain(before.iter_mut()) {
            // Invalid candidates have no completion handle; skip boundedly this round so later valid candidates do not wait forever without wakes.
            for _ in 0..64 {
                let request = match window.next_image_request(now) {
                    Ok(Some(request)) => request,
                    Ok(None) => break,
                    Err(error) => {
                        window.record_image_error(&crate::ui_text::format(
                            "integration-image-request-invalid",
                            &[("error", format!("{error:?}").into())],
                        ));
                        continue;
                    }
                };
                match start(request) {
                    Ok(()) => {
                        window.finish_image_request();
                        break;
                    }
                    Err(StartImageError::Busy) => break,
                    Err(StartImageError::Failed(error)) => {
                        window.finish_image_request();
                        window.record_image_error(&error);
                    }
                }
            }
        }
    }
}
