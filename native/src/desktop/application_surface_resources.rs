//! 主循环结算后安装图片并启动有界 GET；文件信任/DNS/解码仍在既有后台任务。
use super::*;
use crate::surface_resource_runtime::request::Request;
use crate::{desktop::surface_windows::StartImageError, device_authorization_config::Config};
impl Application {
    #[cfg(test)]
    pub(super) fn poll_surface_images_with(
        &mut self,
        mut config: impl FnMut(
            &str,
        )
            -> std::result::Result<Config, crate::device_authorization_config::Error>,
    ) -> Result<()> {
        let now = crate::desktop::surface_windows::now_ms()?;
        self.settle_surface_images(now);
        self.schedule_surface_images_with(&mut config)
    }
    pub(super) fn settle_surface_images(&mut self, now: u64) {
        while self.remote_sessions.deliver_surface_resource(|ready| {
            self.surface_windows.install_ready(ready, now);
            Ok(())
        }) {}
    }
    pub(super) fn schedule_surface_images_with(
        &mut self,
        mut config: impl FnMut(
            &str,
        )
            -> std::result::Result<Config, crate::device_authorization_config::Error>,
    ) -> Result<()> {
        let now = crate::desktop::surface_windows::now_ms()?;
        self.surface_windows.schedule_images(now, |request| {
            let origin = request.binding().origin();
            let lease = self
                .authorization_owner
                .lease(origin, now)
                .map_err(|error| {
                    if error == crate::device_authorization_owner::Error::Capacity {
                        StartImageError::Busy
                    } else {
                        StartImageError::Failed(format!("图片授权不可用：{error:?}"))
                    }
                })?;
            let started = match request {
                Request::Inline(request) => self.remote_sessions.start_inline_image(lease, request),
                Request::Resource(request) => {
                    let config = config(request.binding().origin()).map_err(|error| {
                        StartImageError::Failed(format!("图片配置无效：{error:?}"))
                    })?;
                    self.remote_sessions
                        .start_surface_resource(config, lease, request)
                }
            };
            started.map_err(|error| {
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::AlreadyExists
                ) {
                    StartImageError::Busy
                } else {
                    StartImageError::Failed(format!("图片任务未启动：{error}"))
                }
            })
        });
        Ok(())
    }
}
#[cfg(test)]
#[path = "application_surface_resources_tests.rs"]
mod tests;
