//! 将窗口 owner 与采集 worker 配对；窗口先停止订阅，线程完成由真实句柄唤醒宿主回收。
use super::{window::LiveWindow, Texture};
use crate::live_job::LiveJob;
use std::{io, os::windows::io::BorrowedHandle};
use windows::Win32::Foundation::RECT;
#[path = "composition_live_session_input.rs"]
mod input;
#[path = "composition_live_remote_input.rs"]
mod remote_input;
#[path = "composition_live_source.rs"]
mod source;

pub struct Session {
    input_identity: std::sync::Arc<()>,
    unit_id: crate::workspace_id::UnitId,
    workspace: Option<crate::workspace::IdentityStamp>,
    window: Option<LiveWindow>,
    job: LiveJob<Texture>,
    error: Option<String>,
    stopping: bool,
    capture_result: Option<Result<(), String>>,
    target: Option<crate::live_window_target::Target>,
    input_crop: Option<crate::live_window_target::region::Crop>,
    input:
        Option<crate::live_input_worker::Worker<crate::live_window_target::input_backend::Error>>,
    input_enabled: bool,
    input_error: Option<String>,
    input_cleanup: Option<crate::live_input::Cleanup>,
    input_cleanup_unknown: bool,
    remote_input: Option<remote_input::Remote>,
    last_input_sequence: u64,
    protection: Option<crate::live_visibility_owner::Protection>,
}
impl Session {
    pub(in crate::desktop) fn bind_workspace_chrome(
        &self,
        binding: std::rc::Weak<crate::desktop::application::chrome::Ui>,
    ) {
        if let Some(window) = &self.window {
            window.bind_workspace_chrome(binding);
        }
    }
    /// 窗口目标已经由用户确认；呈现窗口排除成功后才启动后台捕获。
    pub fn start_window(
        target: crate::live_window_target::Target,
        screen: RECT,
    ) -> io::Result<Self> {
        Self::start_window_region(target, None, screen)
    }
    pub fn start_window_region(
        target: crate::live_window_target::Target,
        crop: Option<crate::live_window_target::region::Crop>,
        screen: RECT,
    ) -> io::Result<Self> {
        let (job, window) = LiveJob::start_window_region_bound(target.clone(), crop, |consumer| {
            LiveWindow::new_at(consumer, screen).map_err(io::Error::other)
        })?;
        let mut session = Self::new(job, window);
        if let Some(window) = &session.window {
            window.allow_input().map_err(io::Error::other)?;
        }
        session.target = Some(target);
        session.input_crop = crop;
        Ok(session)
    }
    /// 仅可用于用户已确认的物理选区；不显示窗口，调用方随后显式 show。
    pub fn start_region(screen: RECT) -> io::Result<Self> {
        let (job, window) = LiveJob::start_region_bound(screen, |consumer| {
            LiveWindow::new_at(consumer, screen).map_err(io::Error::other)
        })?;
        Ok(Self::new(job, window))
    }
    pub(in crate::desktop) fn new(job: LiveJob<Texture>, window: LiveWindow) -> Self {
        Self {
            input_identity: std::sync::Arc::new(()),
            unit_id: window.unit_id().clone(),
            workspace: None,
            window: Some(window),
            job,
            error: None,
            stopping: false,
            capture_result: None,
            target: None,
            input_crop: None,
            input: None,
            input_enabled: false,
            input_error: None,
            input_cleanup: None,
            input_cleanup_unknown: false,
            remote_input: None,
            last_input_sequence: 0,
            protection: None,
        }
    }
    pub fn show(&self) -> windows::core::Result<()> {
        self.window
            .as_ref()
            .ok_or_else(|| {
                windows::core::Error::new(
                    windows::Win32::Foundation::E_FAIL,
                    "Live session window is closed",
                )
            })?
            .show()
    }
    pub fn unit_id(&self) -> &crate::workspace_id::UnitId {
        &self.unit_id
    }
    pub(in crate::desktop) fn input_identity(&self) -> std::sync::Arc<()> {
        self.input_identity.clone()
    }
    pub(in crate::desktop) fn bind_workspace(&mut self, workspace: &crate::workspace::Workspace) {
        if self.workspace.is_none() {
            self.workspace = Some(workspace.identity_stamp());
            if let Some(window) = &self.window {
                window.enable_recycle_close();
            }
        }
    }
    pub(in crate::desktop) fn belongs_to(&self, workspace: &crate::workspace::Workspace) -> bool {
        self.workspace
            .as_ref()
            .is_some_and(|stamp| stamp.matches(workspace))
    }
    pub fn is_alive(&self) -> bool {
        !self.stopping && self.window.as_ref().is_some_and(LiveWindow::is_alive)
    }
    pub(in crate::desktop) fn persistence_idle(&self) -> bool {
        self.is_alive()
            && self
                .window
                .as_ref()
                .is_some_and(LiveWindow::persistence_idle)
    }
    pub(in crate::desktop) fn opacity(&self) -> Option<f32> {
        self.is_alive()
            .then(|| self.window.as_ref().unwrap().opacity())
    }
    pub(in crate::desktop) fn persistence_state(&self) -> Option<(RECT, f32)> {
        self.window
            .as_ref()
            .filter(|_| self.is_alive())?
            .persistence_state()
    }
    pub(in crate::desktop) fn exit_idle(&self) -> bool {
        self.is_alive() && self.window.as_ref().is_some_and(LiveWindow::exit_idle)
    }
    pub fn take_freeze_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_freeze_request)
    }
    pub(in crate::desktop) fn take_snapshot_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_snapshot_request)
    }
    pub(in crate::desktop) fn take_link_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_link_request)
    }
    pub(in crate::desktop) fn take_art_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_art_request)
    }
    pub(in crate::desktop) fn art_pending(&self) -> bool {
        self.window.as_ref().is_some_and(LiveWindow::art_pending)
    }
    pub(in crate::desktop) fn art_dialog_owner(&self) -> Option<windows::Win32::Foundation::HWND> {
        self.window
            .as_ref()
            .filter(|_| !self.stopping)?
            .art_dialog_owner()
    }
    pub(in crate::desktop) fn take_unlink_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_unlink_request)
    }
    pub(in crate::desktop) fn take_group_request(&self) -> Option<super::window::groups::Request> {
        self.window
            .as_ref()
            .and_then(LiveWindow::take_group_request)
    }
    pub(in crate::desktop) fn group_input_blocked(&self) -> bool {
        self.stopping
            || self
                .window
                .as_ref()
                .is_none_or(LiveWindow::group_input_blocked)
    }
    pub(in crate::desktop) fn group_dialog_owner(
        &self,
    ) -> Option<windows::Win32::Foundation::HWND> {
        if self.stopping {
            return None;
        }
        self.window
            .as_ref()
            .and_then(LiveWindow::group_dialog_owner)
    }
    pub(in crate::desktop) fn take_input_notice(&self) -> Option<windows::Win32::Foundation::HWND> {
        self.window
            .as_ref()
            .filter(|window| window.take_input_notice())
            .and_then(LiveWindow::group_dialog_owner)
    }
    pub(in crate::desktop) fn sync_groups(
        &self,
        workspace: &crate::workspace::Workspace,
    ) -> windows::core::Result<()> {
        if self.belongs_to(workspace) {
            if let Some(window) = &self.window {
                window.sync_groups(workspace)?;
            }
        }
        Ok(())
    }
    pub fn take_recycle_request(&self) -> bool {
        self.window
            .as_ref()
            .is_some_and(LiveWindow::take_recycle_request)
    }
    pub(in crate::desktop) fn close_for_recycle(&mut self) -> windows::core::Result<()> {
        self.window
            .as_ref()
            .ok_or_else(|| {
                windows::core::Error::new(
                    windows::Win32::Foundation::E_ABORT,
                    "Live recycle session closed",
                )
            })?
            .close_for_recycle()?;
        self.stop();
        Ok(())
    }
    /// 显式像素边界；不停止采集/输入，转换成功后的提交仍由应用事务拥有。
    pub fn snapshot_frame(
        &self,
    ) -> windows::core::Result<Option<std::sync::Arc<crate::live_frames::Frame<Texture>>>> {
        match self.window.as_ref().filter(|_| !self.stopping) {
            Some(window) => window.snapshot_frame(),
            None => Ok(None),
        }
    }
    pub fn freeze_rect(
        &self,
        source: [u32; 2],
    ) -> windows::core::Result<crate::pin_geometry::PinRect> {
        self.window
            .as_ref()
            .filter(|_| !self.stopping)
            .ok_or_else(|| {
                windows::core::Error::new(
                    windows::Win32::Foundation::E_ABORT,
                    "Live session closed",
                )
            })?
            .freeze_rect(source)
    }
    pub(crate) fn protect(&mut self, protection: crate::live_visibility_owner::Protection) {
        self.protection = Some(protection);
    }
    /// 加入宿主消息等待集合；源 close 通知可能早于线程退出，此句柄不会提前报完成。
    pub fn completion_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.job.completion_handle()
    }
    pub fn source_process_handle(&self) -> Option<BorrowedHandle<'_>> {
        self.job.source_process_handle()
    }
    /// 显式停止不在 UI 回调中 join；保留 owner 到 poll 领取结果或应用最终析构。
    pub fn stop(&mut self) {
        self.stopping = true;
        self.cancel_remote_input();
        self.disable_input();
        if self.input.is_none() {
            self.protection.take();
        }
        self.window.take();
        self.job.cancel();
    }
    /// 每次消息分发后及线程句柄就绪时调用；返回值只交付一次，无定时轮询。
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        self.poll_input();
        self.poll_remote_input();
        self.process_window_input();
        self.job.refresh_source();
        if let Some(window) = &self.window {
            if let Err(error) = window.refresh() {
                self.error.get_or_insert_with(|| error.to_string());
            }
            if let Some(error) = window.take_error() {
                self.error.get_or_insert_with(|| error.to_string());
            }
            if !window.is_alive() || self.error.is_some() {
                // 不主动 cancel：源失败关闭窗口时必须保留 worker 的原始错误。
                self.window.take();
                self.cancel_remote_input();
                self.disable_input();
            }
        }
        if self.capture_result.is_none() {
            self.capture_result = self.job.take_ready();
        }
        if self.capture_result.is_some() {
            self.window.take();
            self.cancel_remote_input();
            self.disable_input();
        }
        if self.window.is_none() && self.input.is_none() {
            self.protection.take();
        }
        if self.capture_result.is_none() {
            // 输出已关闭或任务已完成时不再隐藏源，避免迟到 ACK 引发隐藏/恢复闪烁。
            if let Some(protection) = self.protection.as_ref().filter(|_| self.window.is_some()) {
                if let Err(error) = protection.poll() {
                    self.error.get_or_insert_with(|| error.to_string());
                    self.window.take();
                    self.disable_input();
                    if self.input.is_none() {
                        self.protection.take();
                    }
                    self.job.cancel();
                }
            }
            return None;
        }
        if self.input.is_some() {
            return None;
        }
        if self.input_cleanup_unknown || self.input_cleanup.is_some_and(|report| report.failed > 0)
        {
            self.error
                .get_or_insert_with(|| "Live input cleanup failed or outcome unknown".into());
        }
        let result = self.capture_result.take()?;
        self.protection.take();
        self.window.take();
        Some(if let Some(error) = self.error.take() {
            Err(error)
        } else if self.stopping {
            Ok(())
        } else {
            result
        })
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
        // LiveJob 的最终析构仍 join；不能用 detach 隐藏不可中断驱动调用。
    }
}

#[cfg(test)]
#[path = "composition_live_input_runtime_tests.rs"]
mod input_runtime_tests;
#[cfg(test)]
#[path = "composition_live_session_tests.rs"]
mod tests;
