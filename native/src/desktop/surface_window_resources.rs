//! 资源请求与像素消费使用当前授权窗口快照；不在这里发请求。
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
    /// 隐藏待激活窗口也可准备 GET；完整 Binding 取当前生命周期版本而非初始 mount。
    pub fn image_resource(
        &mut self,
        id: &str,
        now: u64,
    ) -> std::result::Result<ResourceLease, ImageError> {
        self.resource_mount(now)?
            .image_resource(id, now)
            .map_err(ImageError::Resource)
    }
    /// 仅交付共享像素；安装方在任何可重入 HWND 调用之后仍须重新检查当前版本与授权。
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
