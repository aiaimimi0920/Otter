//! Resource requests/pixel consumption use the current authorized-window snapshot; send no requests here.
use super::*;
use crate::{
    live_relay_http::{MountedSurface, ResourceError, ResourceLease},
    surface_resource_runtime::Ready,
};
#[path = "surface_window_image_install.rs"]
mod install;
pub(super) use install::InstalledImages;
#[path = "surface_window_image_requests.rs"]
mod requests;
pub(super) use requests::Requests;
#[derive(Debug)]
pub enum ImageError {
    Window(Error),
    Inactive,
    Snapshot(crate::surface_resource_snapshot::Error),
    Resource(ResourceError),
    Ready(crate::surface_resource_runtime::Error),
}
impl AuthorizedWindow {
    fn resource_mount(&mut self, now: u64) -> std::result::Result<MountedSurface, ImageError> {
        self.refresh(now).map_err(ImageError::Window)?;
        if self.javascript.is_some() || self.stopped.is_some() || !self.is_alive() {
            return Err(ImageError::Inactive);
        }
        self.resources
            .borrow()
            .as_ref()
            .ok_or(ImageError::Inactive)?
            .mount(self.binding(), self.snapshot_revision)
            .map_err(ImageError::Snapshot)
    }
    /// Hidden pending-activation windows may also prepare GET; complete Binding uses current lifecycle version, not initial mount.
    pub fn image_resource(
        &mut self,
        id: &str,
        now: u64,
    ) -> std::result::Result<ResourceLease, ImageError> {
        self.resource_mount(now)?
            .image_resource(id, now)
            .map_err(ImageError::Resource)
    }
    /// Deliver shared pixels only; installers still recheck current version/authorization after any reentrant HWND call.
    pub fn resource_image(
        &mut self,
        ready: &Ready,
        now: u64,
    ) -> std::result::Result<crate::image_decode::Image, ImageError> {
        let current = self.resource_mount(now)?;
        ready
            .image(&current, now)
            .cloned()
            .map_err(ImageError::Ready)
    }
}
