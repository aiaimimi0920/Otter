//! 原 AuthorizedWindow 的 JS 执行接缝；UI 只排命令/取完成帧，不取码、求值或 join 活线程。
use super::*;
use crate::{
    live_relay_http::MountedSurface, surface_javascript::authorized::Runtime,
    surface_live_binding::Binding,
};
#[path = "surface_window_javascript_animation.rs"]
mod animation;
#[path = "surface_window_javascript_input.rs"]
mod input;
#[path = "surface_window_javascript_presentation.rs"]
mod presentation;
#[derive(Default)]
pub(super) struct Execution {
    runtime: Option<Runtime>,
    synced: Option<(Binding, u64)>,
    pub(super) events: Option<Vec<crate::surface_click_event::ClickEvent>>,
    // emit 暂存、取出和 cancel 返还共享此依赖；只由下一批真实 listener 帧覆盖。
    events_viewport: Option<crate::surface_viewport::Viewport>,
    events_viewport_epoch: Option<u64>,
    submitted_viewport_epoch: u64,
    submitted_focus_epoch: u64,
    // 只有本地输入 carrier 的 ticket；与 listener emit 的远端 batch 身份严格分离。
    local_input: Option<String>,
    local_pointer: Option<crate::surface_viewport::Viewport>,
    staged: Option<(crate::surface_resource_snapshot::ResourceSnapshot, u64)>,
    presentation: Option<u64>,
    animation_due: Option<u64>,
    timer_due: Option<u64>,
    resize_observed: bool,
    observed_viewport_epoch: u64,
}
impl AuthorizedWindow {
    fn javascript_mount(&self) -> Result<MountedSurface> {
        if let Some((resources, revision)) =
            self.javascript.as_ref().and_then(|js| js.staged.as_ref())
        {
            return resources
                .mount(self.binding(), *revision)
                .map_err(|_| Error::new(E_FAIL, "JS staged snapshot identity changed"));
        }
        self.resources
            .borrow()
            .as_ref()
            .ok_or_else(|| Error::new(E_FAIL, "JS snapshot unavailable"))?
            .mount(self.binding(), self.snapshot_revision)
            .map_err(|_| Error::new(E_FAIL, "JS snapshot identity changed"))
    }
    pub(super) fn javascript_ready(&self) -> bool {
        self.javascript.as_ref().is_none_or(|js| {
            js.staged.is_none()
                && js
                    .runtime
                    .as_ref()
                    .is_some_and(|runtime| !runtime.pending() && !runtime.finished())
                && js.synced.as_ref().is_some_and(|(binding, revision)| {
                    binding == self.binding() && *revision == self.snapshot_revision
                })
        })
    }
    pub(super) fn javascript_deadline(&self) -> u64 {
        self.javascript
            .as_ref()
            .and_then(|js| js.runtime.as_ref())
            .map_or(u64::MAX, Runtime::expires_at_ms)
    }
    pub(super) fn stop_javascript(&self) {
        if let Some(runtime) = self.javascript.as_ref().and_then(|js| js.runtime.as_ref()) {
            runtime.request_stop();
        }
    }
    pub(crate) fn javascript_finished(&self) -> bool {
        self.javascript
            .as_ref()
            .and_then(|js| js.runtime.as_ref())
            .is_none_or(Runtime::finished)
    }
    /// 未启动的 JS 同样预留回复/完成槽；不能只数当前实际句柄后允许 Live 吃掉未来容量。
    pub(crate) fn wait_budget(&self) -> usize {
        if self.is_alive() {
            1 + usize::from(self.javascript.is_some()) * 2
        } else {
            usize::from(!self.javascript_finished())
        }
    }
    pub(crate) fn javascript_wait_handles(&self) -> impl Iterator<Item = BorrowedHandle<'_>> {
        let alive = self.is_alive();
        self.javascript
            .as_ref()
            .and_then(|js| js.runtime.as_ref())
            .into_iter()
            .flat_map(Runtime::wait_handles)
            .enumerate()
            // 关闭后未消费帧可能仍有信号；只等真实线程终态，避免退出泵忙循环。
            .filter_map(move |(index, handle)| (alive || index == 0).then_some(handle))
    }
    pub(crate) fn javascript_needs_start(&self) -> bool {
        self.is_alive()
            && self.activation_error.is_none()
            && self
                .javascript
                .as_ref()
                .is_some_and(|js| js.runtime.is_none())
    }
    pub(crate) fn start_javascript(
        &mut self,
        config: crate::device_authorization_config::Config,
        source: Lease,
    ) -> std::io::Result<()> {
        if !self.javascript_needs_start() {
            return Err(std::io::ErrorKind::AlreadyExists.into());
        }
        let current = self.javascript_mount().map_err(std::io::Error::other)?;
        let viewport = self
            .capture_javascript_viewport()
            .map_err(std::io::Error::other)?;
        let runtime = Runtime::start(config, source, &self.lease, current, viewport)?;
        self.javascript.as_mut().unwrap().runtime = Some(runtime);
        Ok(())
    }
    pub(crate) fn fail_javascript(&mut self, error: String) -> Result<()> {
        self.record_activation_error(error);
        self.close()
    }
    pub(super) fn poll_javascript(&mut self, now: u64) -> Result<()> {
        if let Err(error) = self.settle_javascript(now) {
            // 呈现/快照/排队失败均为终态，不能在下一轮用空 rebind 回复假称帧已呈现。
            self.fail_javascript(format!("JS 结算失败：{error}"))?;
        }
        Ok(())
    }
    fn settle_javascript(&mut self, now: u64) -> Result<()> {
        if self.javascript.is_none() {
            return Ok(());
        }
        let alive = self.is_alive();
        let js = self.javascript.as_mut().unwrap();
        let Some(runtime) = js.runtime.as_mut() else {
            return Ok(());
        };
        if let Some(exit) = runtime.take_exit() {
            if alive {
                self.fail_javascript(format!("JS 执行已结束：{exit:?}"))?;
            }
            return Ok(());
        }
        if !alive || runtime.finished() {
            return Ok(());
        }
        if now >= runtime.expires_at_ms() {
            return self.fail_javascript("JS 资源或凭据已过期".into());
        }
        // 常见空闲/等待消息不复制或解析最多 1 MiB 的快照；事件/线程句柄负责下一次唤醒。
        if (runtime.pending() && !runtime.reply_ready())
            || (!runtime.pending() && self.javascript_ready())
        {
            return Ok(());
        }
        let current = self.javascript_mount()?;
        let js = self.javascript.as_mut().unwrap();
        let runtime = js.runtime.as_mut().unwrap();
        if let Some(reply) = runtime.take_reply(&self.lease, &current) {
            match reply {
                Ok(frame) => {
                    if let Some(frame) = frame {
                        self.accept_javascript_frame(frame)?;
                    }
                    // replace 可同步重入；交付后再验原租约和窗口。
                    if self.lease.validate(current.binding.origin(), now).is_err()
                        || !self.is_alive()
                    {
                        self.close()?;
                        return Ok(());
                    }
                    self.javascript.as_mut().unwrap().synced =
                        Some((current.binding.clone(), self.snapshot_revision));
                }
                Err(error) => return self.fail_javascript(format!("JS 帧未接纳：{error:?}")),
            }
        }
        let js = self.javascript.as_mut().unwrap();
        let runtime = js.runtime.as_mut().unwrap();
        if !runtime.pending()
            && js.synced.as_ref() != Some(&(current.binding.clone(), self.snapshot_revision))
        {
            runtime
                .rebind(current)
                .map_err(|e| Error::new(E_FAIL, e.to_string()))?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn javascript_wait_reply_for_test(&mut self) {
        self.javascript
            .as_mut()
            .unwrap()
            .runtime
            .as_mut()
            .unwrap()
            .wait_reply_for_test()
            .expect("JS runtime exited before its expected reply");
    }
    #[cfg(test)]
    pub(crate) fn javascript_pending_for_test(&self) -> bool {
        self.javascript
            .as_ref()
            .and_then(|js| js.runtime.as_ref())
            .is_some_and(Runtime::pending)
    }
    #[cfg(test)]
    pub(crate) fn javascript_text_for_test(&self) -> String {
        self.window.javascript_text_for_test()
    }
    #[cfg(test)]
    pub(crate) fn javascript_resize_report_for_test(&self) -> serde_json::Value {
        self.window.javascript_resize_report_for_test()
    }
    #[cfg(test)]
    pub(crate) fn javascript_canvas_pixel_for_test(&self, x: usize, y: usize) -> [u8; 4] {
        self.window.javascript_canvas_pixel_for_test(x, y)
    }
    #[cfg(test)]
    pub(crate) fn javascript_pointer_message_for_test(&self, message: u32, offset: i32) {
        self.window
            .javascript_pointer_message_for_test(message, offset);
    }
    #[cfg(test)]
    pub(crate) fn javascript_hover_message_for_test(&self, message: u32) {
        self.window.javascript_hover_message_for_test(message);
    }
    #[cfg(test)]
    pub(crate) fn javascript_click_for_test(&self) {
        self.window
            .javascript_queue_click_for_test(&self.window.javascript_first_button_for_test());
    }
    #[cfg(test)]
    pub(crate) fn javascript_focus_request_for_test(&self, focus: bool) {
        self.window.javascript_focus_request_for_test(focus);
    }
    #[cfg(test)]
    pub(crate) fn javascript_probe(
        &self,
    ) -> Option<std::sync::Arc<crate::bounded_worker_process::Probe>> {
        self.javascript.as_ref()?.runtime.as_ref()?.probe()
    }
}
