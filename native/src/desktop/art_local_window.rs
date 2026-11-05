//! 本地签名 Art 的 HWND 呈现 owner；不持 Local/child，不伪造旧远端附件生命周期。
//! 宿主每次 poll 提供当前选择/节点凭证；可重入 Win32 提交前后都复核，关闭先撤权后等待后台终态。
use super::surface_window::SurfaceWindow;
use crate::{art_execution_grant::wire::Binding, surface_javascript::signed::runtime::Runtime};
#[cfg(test)]
use std::os::windows::io::BorrowedHandle;
use windows::core::{Error, Result};
use windows::Win32::Foundation::E_FAIL;
#[path = "art_local_window_confirmation.rs"]
mod confirmation;
#[path = "art_local_confirmation.rs"]
mod confirmation_dialog;
pub(in crate::desktop) use confirmation_dialog::translate_confirmation;
#[path = "art_local_window_input.rs"]
mod input;
#[path = "art_local_window_present.rs"]
mod present;
#[path = "art_local_trigger.rs"]
mod trigger;
pub(super) struct Window {
    runtime: Runtime,
    viewer: Option<std::sync::Arc<crate::live_relay_http::art_prepare::viewer::Proof>>,
    current: Binding,
    window: Option<SurfaceWindow>,
    stopping: bool,
    shown: bool,
    has_frame: bool,
    input: input::Input,
    status: Option<String>,
    service_progress_visible: bool,
    result: Option<crate::surface_javascript_wire::result::Resolved>,
    confirmation: Option<confirmation_dialog::Prompt>,
    trigger: Option<crate::art_live_trigger::ticket::Ticket>,
    trigger_sent: bool,
    formal_revision: u64,
}
impl Window {
    #[cfg(test)]
    pub(super) fn test_result_ready(&self) -> bool {
        self.result.is_some()
    }
    #[cfg(test)]
    pub(super) fn test_window(&self) -> Option<&SurfaceWindow> {
        self.window.as_ref()
    }
    pub fn new(runtime: Runtime, view: crate::surface_live_binding::Binding) -> Self {
        Self {
            current: runtime.binding().clone(),
            runtime,
            viewer: None,
            window: None,
            stopping: false,
            shown: false,
            has_frame: false,
            input: input::Input::new(view),
            status: None,
            service_progress_visible: false,
            result: None,
            confirmation: None,
            trigger: None,
            trigger_sent: false,
            formal_revision: 0,
        }
    }
    pub fn start_with_content(
        prepared: crate::live_relay_http::art_prepare::Prepared,
        view: crate::surface_live_binding::Binding,
        notify: std::sync::Arc<dyn Fn() + Send + Sync>,
        content: crate::surface_javascript::signed::runtime::Content,
    ) -> Result<Self> {
        // 先创建隐藏宿主再启动 child；首帧接纳前不显示空壳，也不开放输入。
        let scene =
            crate::surface_scene::Scene::parse(br#"{"id":"root","type":"column","children":[]}"#)
                .map_err(|_| Error::new(E_FAIL, "Local Art initial scene invalid"))?;
        let window = SurfaceWindow::new(scene, 640, 480)?;
        window.enable_javascript_keyboard();
        window.bind_edits(&view)?;
        window.set_input_policy(false, false)?;
        let viewport = window.viewport()?;
        let online = prepared.online();
        let viewer = prepared.viewer_proof();
        let runtime = Runtime::start_content_notified(
            prepared,
            view.clone(),
            content,
            online,
            Some(viewport),
            notify,
        )
        .map_err(|_| Error::new(E_FAIL, "Local Art worker could not start"))?;
        let mut owner = Self::new(runtime, view);
        owner.viewer = viewer;
        owner.input.viewport = Some(viewport);
        owner.input.epoch = window.viewport_epoch();
        owner.input.focus = window.javascript_focus_epoch();
        owner.window = Some(window);
        Ok(owner)
    }
    fn current(&self, host: &impl Fn() -> bool) -> bool {
        !self.stopping
            && host()
            && self.runtime.validate(&self.current).is_ok()
            && (!self.trigger_sent
                || self
                    .trigger
                    .as_ref()
                    .is_some_and(|ticket| ticket.current(&self.current)))
    }
    #[cfg(test)]
    pub fn wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        self.runtime.wait_handles()
    }
    pub fn finished(&self) -> bool {
        self.runtime.finished()
    }
    pub fn ready(&self) -> bool {
        !self.stopping
            && self.has_frame
            && self.window.as_ref().is_some_and(SurfaceWindow::is_alive)
    }
    /// 未交付结果/确认/输入及运行中请求必须先沿原泵结算；定时器本身不等于已接受输入。
    pub(super) fn session_end_idle(&self) -> bool {
        self.ready()
            && !self.runtime.pending()
            && self.confirmation.is_none()
            && self.result.is_none()
            && self.trigger.is_none()
            && self.input.deferred.is_none()
            && self.input.ticket.is_none()
            && self
                .window
                .as_ref()
                .is_some_and(SurfaceWindow::session_end_idle)
    }
    pub(super) fn viewer_proof(
        &self,
    ) -> Option<std::sync::Arc<crate::live_relay_http::art_prepare::viewer::Proof>> {
        if !self.ready() || !self.current(&|| true) {
            return None;
        }
        let proof = self.viewer.as_ref()?;
        proof.validate().ok()?;
        Some(proof.clone())
    }
    pub fn take_status(&mut self) -> Option<String> {
        self.status.take()
    }
    pub(super) fn take_result(
        &mut self,
        host: &impl Fn() -> bool,
    ) -> Option<crate::surface_javascript_wire::result::Resolved> {
        if !self.current(host) {
            self.result = None;
            return None;
        }
        self.result.take()
    }
    pub fn stop(&mut self) -> Result<()> {
        self.complete_trigger(crate::art_live_trigger::Receipt::Cancelled);
        self.stopping = true;
        self.viewer = None;
        self.input.clear();
        self.status = None;
        self.service_progress_visible = false;
        self.result = None;
        self.runtime.request_stop();
        self.confirmation.take();
        if let Some(window) = &self.window {
            window.close()?;
        }
        Ok(())
    }
    pub fn poll(&mut self, host: impl Fn() -> bool, visible: bool) -> Result<()> {
        if !host()
            || self
                .window
                .as_ref()
                .is_some_and(|window| !window.is_alive())
        {
            self.stop()?;
        }
        if let Some(result) = self.runtime.take_exit() {
            let expected = self.stopping;
            if result.is_err() {
                self.complete_trigger(crate::art_live_trigger::Receipt::Failed);
            }
            self.stop()?;
            return if result.is_err() && !expected {
                Err(Error::new(E_FAIL, "Local Art execution stopped"))
            } else {
                Ok(())
            };
        }
        if self.stopping {
            return Ok(());
        }
        if self.runtime.validate(&self.current).is_err() {
            self.stop()?;
            return Err(Error::new(
                E_FAIL,
                "Local Art authorization expired or revoked",
            ));
        }
        self.poll_confirmation(&host, visible)?;
        if let Some(progress) = self.runtime.take_progress(&self.current) {
            self.service_progress_visible = true;
            use crate::live_relay_http::art_prepare::service::Status;
            self.status = Some(match progress.status {
                Status::Accepted => "Art 服务已排队".into(),
                Status::Running => progress.percent.map_or_else(
                    || "Art 服务执行中".into(),
                    |percent| format!("Art 服务执行中 {percent}%"),
                ),
                Status::Succeeded => "Art 服务完成".into(),
                Status::Failed => "Art 服务失败".into(),
                Status::Cancelled => "Art 服务已取消".into(),
            });
        }
        let Some(frame) = self.runtime.take_reply(&self.current) else {
            let result = self
                .show_if_needed(&host, visible)
                .and_then(|()| self.advance(&host, visible));
            if result.is_err() {
                self.stop()?;
            }
            return result;
        };
        let result = (|| {
            let frame = frame.map_err(|_| Error::new(E_FAIL, "Local Art frame rejected"))?;
            if !self.current(&host) {
                return Err(Error::new(E_FAIL, "Local Art host changed"));
            }
            self.present(frame, &host, visible)?;
            if self.trigger_sent && self.result.is_none() {
                self.complete_trigger(crate::art_live_trigger::Receipt::NoResult);
            }
            self.show_if_needed(&host, visible)?;
            self.advance(&host, visible)
        })();
        if result.is_ok() && self.service_progress_visible {
            self.service_progress_visible = false;
            self.status = Some("Art 就绪".into());
        }
        if result.is_err() {
            self.complete_trigger(crate::art_live_trigger::Receipt::Failed);
            self.stop()?;
        }
        result
    }
    fn show_if_needed(&mut self, host: &impl Fn() -> bool, visible: bool) -> Result<()> {
        let Some(window) = &self.window else {
            return Ok(());
        };
        if !visible || self.shown || !self.has_frame {
            return Ok(());
        }
        if !window.is_alive() || !self.current(host) {
            return Err(Error::new(E_FAIL, "Local Art host changed"));
        }
        // 首帧可在隐藏期间领取；首次可见不依赖新帧，之后不覆盖用户最小化/隐藏意图。
        window.show()?;
        if !window.is_alive() || !self.current(host) {
            return Err(Error::new(E_FAIL, "Local Art host changed"));
        }
        self.shown = true;
        Ok(())
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
#[cfg(test)]
#[path = "art_local_window_tests.rs"]
mod tests;
