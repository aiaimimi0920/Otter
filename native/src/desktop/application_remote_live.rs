//! 远程会话在应用消息结算点回收；窗口回调不显示终态对话框、不等待后台线程。
use super::*;
#[path = "application_surface_javascript.rs"]
mod javascript;
#[path = "application_surface_resources.rs"]
mod resources;
impl Application {
    pub(super) unsafe fn poll_remote_live(&mut self, visible: bool) -> Result<()> {
        self.poll_remote_live_with(
            visible,
            crate::device_authorization_config::Config::from_env,
        )
    }
    unsafe fn poll_remote_live_with(
        &mut self,
        visible: bool,
        mut config: impl FnMut(
            &str,
        ) -> std::result::Result<
            crate::device_authorization_config::Config,
            crate::device_authorization_config::Error,
        >,
    ) -> Result<()> {
        self.poll_live_sources_with(&mut config);
        self.remote_sessions
            .expire_surface_resources(crate::desktop::surface_windows::now_ms()?);
        let windows = &self.surface_windows;
        self.remote_sessions
            .cancel_surface_resources(|binding, revision| {
                !windows.retains_resource(binding, revision)
            });
        for (origin, error) in self.remote_sessions.poll_surfaces() {
            let message = format!("Surface 操作未完成。\n{origin}\n{error:?}");
            if !visible {
                return Err(Error::new(E_FAIL, message));
            }
            MessageBoxW(
                None,
                &HSTRING::from(message),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
        self.poll_surface_recovery_ui();
        self.admit_surface_results()?;
        if let Some(error) = self.remote_sessions.poll_catalog() {
            let mut view = self.authorization.borrow_mut();
            view.catalog_busy = false;
            view.catalog = None;
            view.status = format!("目录查询失败：{error:?}");
            drop(view);
            let message = format!("无法获取远程会话目录。\n{error:?}");
            if !visible {
                return Err(Error::new(E_FAIL, message));
            }
            MessageBoxW(
                None,
                &HSTRING::from(message),
                w!("Otter"),
                MB_OK | MB_ICONWARNING,
            );
        }
        if let Some(catalog) = self.remote_sessions.take_catalog() {
            let mut view = self.authorization.borrow_mut();
            if !view.closed && view.origin.as_deref() == Some(&catalog.origin) {
                view.catalog_busy = false;
                view.status = format!("远程会话目录：{} 项", catalog.entries.len());
                view.catalog = Some(catalog);
            }
        }
        if !self.remote_sessions.discovering() {
            let mut view = self.authorization.borrow_mut();
            if view.catalog_busy {
                view.catalog_busy = false;
                view.status = "目录查询已取消，请重新获取授权后刷新".into();
            }
        }
        self.settle_live_viewers()?;
        for event in self.remote_sessions.poll() {
            self.authorization.borrow_mut().live.status = "远程观看已结束".into();
            if let Some(error) = event.outcome.error_message() {
                let message = format!("远程观看已停止。\n{}\n{error}", event.origin);
                if !visible {
                    return Err(Error::new(E_FAIL, message));
                }
                MessageBoxW(
                    None,
                    &HSTRING::from(message),
                    w!("Otter"),
                    MB_OK | MB_ICONWARNING,
                );
            }
        }
        // 先回收目录/观看/图片完成槽，再安排 JS；最后才结算生命周期，保证后台失败
        // 或同步 start 失败关闭的 HWND 在本轮进入 inactive，不丢失最后一个唤醒源。
        self.settle_surface_images(crate::desktop::surface_windows::now_ms()?);
        self.poll_surface_javascript_with(visible, &mut config)?;
        self.activate_surface_windows_with(visible, &mut config)?;
        self.settle_live_viewers()?;
        let windows = &self.surface_windows;
        self.remote_sessions
            .cancel_surface_resources(|binding, revision| {
                !windows.retains_resource(binding, revision)
            });
        {
            let mut view = self.authorization.borrow_mut();
            view.mount_busy = self.remote_sessions.attaching() || self.local_arts.busy();
            if view.restoring && !view.mount_busy {
                view.restoring = false;
                view.recovery_status.clear();
            }
            if !view.mount_busy && !view.busy {
                if let Some(status) = self.surface_windows.status() {
                    view.status = status;
                }
            }
        }
        self.schedule_surface_images_with(&mut config)?;
        Ok(())
    }
}
