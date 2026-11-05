//! 认证挂载结果的窗口 owner；本地关闭不冒充服务端 disposed，不自动重放请求。
use super::surface_window::SurfaceWindow;
use crate::{
    device_authorization_lease::{Error as LeaseError, Lease, Notification},
    surface_attach_runtime::{AdmissionError, Outcome},
    surface_lifecycle_owner::Owner,
};
use std::os::windows::io::BorrowedHandle;
use windows::core::{Error, Result};
use windows::Win32::Foundation::E_FAIL;
#[path = "surface_confirmation_dialog.rs"]
pub(super) mod confirmation_dialog;
#[path = "surface_receipt_decision.rs"]
mod receipt_decision;
pub use receipt_decision::ReceiptDecisionError;
#[path = "surface_batch_delivery.rs"]
mod batch_delivery;
pub use batch_delivery::BatchReceipt;
#[path = "surface_batch_confirmation.rs"]
mod batch_confirmation;
pub use batch_confirmation::{BatchDecisionError, PreparedBatchDecision};
#[path = "surface_batch_decision_transfer.rs"]
mod batch_decision_transfer;
pub use batch_decision_transfer::{BatchDecisionInput, BatchDecisionStartError};
#[path = "surface_batch_continuation.rs"]
mod batch_continuation;
pub use batch_continuation::BatchContinuationInput;
#[path = "surface_event_delivery.rs"]
mod event_delivery;
pub use event_delivery::{EventReceipt, PresentationError};
#[path = "surface_window_resources.rs"]
mod resources;
pub use resources::ImageError;
#[path = "surface_window_admission.rs"]
mod admission;
#[path = "surface_window_javascript.rs"]
mod javascript;

#[derive(Debug)]
pub enum OpenError {
    Admission(AdmissionError),
    Notification(std::io::Error),
    Window(Error),
}
pub struct AuthorizedWindow {
    window: SurfaceWindow,
    lease: Lease,
    notice: Notification,
    lifecycle: crate::surface_activation::Activation,
    stopped: Option<LeaseError>,
    request: std::sync::Arc<crate::surface_attach_runtime::Request>,
    activation_error: Option<String>,
    rejected_owner: Option<Box<Owner>>,
    shown: bool,
    snapshot_revision: u64,
    result_binding_pending: bool,
    deferred_click: Option<super::surface_window::DeferredClick>,
    resources: std::cell::RefCell<Option<crate::surface_resource_snapshot::ResourceSnapshot>>,
    images: std::cell::RefCell<resources::InstalledImages>,
    image_requests: std::cell::RefCell<resources::Requests>,
    javascript: Option<javascript::Execution>,
}
impl AuthorizedWindow {
    pub fn is_alive(&self) -> bool {
        self.window.is_alive()
    }
    #[cfg(test)]
    pub(crate) fn edit_test_parent(&self) -> windows::Win32::Foundation::HWND {
        self.window.dialog_owner().unwrap()
    }
    pub fn is_shown(&self) -> bool {
        self.shown && self.is_alive()
    }
    pub fn lifecycle(&self) -> Option<&Owner> {
        self.lifecycle.owner()
    }
    pub fn binding(&self) -> &crate::surface_live_binding::Binding {
        self.lifecycle.binding()
    }
    /// 当前呈现场景的快照版本；生命周期 ACK 不能推进场景版本。
    pub fn snapshot_revision(&self) -> u64 {
        self.snapshot_revision
    }
    /// 仅新挂载窗口拥有一次换绑资格，普通结果不能重置此标记。
    pub fn result_binding_pending(&self) -> bool {
        self.result_binding_pending
    }
    pub fn bind_node_results(
        &mut self,
        now_ms: u64,
        bind: impl FnOnce(&crate::surface_live_binding::Binding) -> Result<bool>,
    ) -> Result<bool> {
        if !self.result_binding_pending {
            return Ok(false);
        }
        self.refresh(now_ms)?;
        if self.stopped.is_some()
            || !self.is_shown()
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Ok(false);
        }
        // refresh 可能同步关闭 HWND；节点/View 借用只能在刷新后的回调内获取。
        let bound = bind(self.binding())?;
        if bound {
            self.result_binding_pending = false;
        }
        Ok(bound)
    }
    /// 准备前后检查授权与当前呈现身份；未发送，调用 begin_event 时仍须再次复核。
    pub fn prepare_click(
        &mut self,
        click: super::surface_window::Click,
        now_ms: u64,
    ) -> Result<Vec<crate::surface_click_event::ClickEvent>> {
        self.refresh(now_ms)?;
        if self.javascript.is_some()
            || self.stopped.is_some()
            || !self.is_shown()
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Err(Error::new(E_FAIL, "Surface input is not authorized"));
        }
        let binding = self.binding().clone();
        let revision = self.snapshot_revision;
        let events = self
            .window
            .click_events_for_candidate(click, &binding, revision)?;
        self.refresh(now_ms)?;
        if self.stopped.is_some()
            || !self.is_shown()
            || self.binding() != &binding
            || self.snapshot_revision != revision
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Err(Error::new(E_FAIL, "Surface input became stale"));
        }
        Ok(events)
    }
    /// 无候选时避免在主循环反复准备租约和客户端配置。
    pub fn has_pending_click(&self) -> bool {
        if self.javascript.is_some() {
            return self.javascript_has_input();
        }
        self.deferred_click.is_some()
            || self.window.has_pending_click()
            || self.window.has_pending_edit()
    }
    /// 应用意图不是授权；实际显示与唯一 owner 状态共同决定本地输入资格。
    pub fn sync_pointer(&mut self, requested: bool, now_ms: u64) -> Result<bool> {
        self.sync_input(requested, requested, now_ms)
    }
    /// 本地编辑可跨动作 owner 移交存活；提交资格仍要求唯一已确认 owner 和应用门禁。
    pub fn sync_input(&mut self, editing: bool, submitting: bool, now_ms: u64) -> Result<bool> {
        self.refresh(now_ms)?;
        // JS 仅在 listener/update/呈现收据均空闲时捕获下一候选，emit 仍经原 batch FIFO。
        let editing = editing
            && self.stopped.is_none()
            && self.is_shown()
            && self.lifecycle.local_edit_active();
        let enabled = editing
            && submitting
            && self.javascript_input_idle()
            && self.lifecycle.owner().is_some_and(Owner::confirmed_active);
        if !editing {
            self.deferred_click = None;
        }
        if self.is_alive() {
            self.window.set_input_policy(editing, enabled)?;
        }
        Ok(enabled && !self.schedule_javascript_animation(now_ms)?)
    }
    pub fn local_edit_enabled(&self) -> bool {
        self.window.local_edit_enabled()
    }
    pub(crate) fn disable_local_input(&mut self) -> Result<()> {
        self.deferred_click = None;
        if self.is_alive() {
            self.window.set_input_policy(false, false)?;
        }
        Ok(())
    }
    /// 字段前缀经同一授权门禁准备；到达点击 fence 后不得先派发更晚的 change。
    pub fn take_prepared_click(
        &mut self,
        now_ms: u64,
    ) -> Result<Option<Vec<crate::surface_click_event::ClickEvent>>> {
        if self.javascript.is_some() {
            return self.take_javascript_input(now_ms);
        }
        if self.deferred_click.is_none() {
            if let Some(click) = self.window.take_click() {
                self.deferred_click = Some(self.window.defer_click(
                    click,
                    self.binding(),
                    self.snapshot_revision,
                )?);
            }
        }
        if let Some(pending) = &self.deferred_click {
            use super::surface_edit_mount::FenceState;
            match self.window.deferred_state(pending) {
                FenceState::Invalid => {
                    self.deferred_click = None;
                    return Err(Error::new(
                        E_FAIL,
                        "Surface click dependency invalidated or expired",
                    ));
                }
                FenceState::InFlight => return Ok(None),
                FenceState::Ready => {
                    let pending = self.deferred_click.take().unwrap();
                    self.refresh(now_ms)?;
                    if self.stopped.is_some()
                        || !self.is_shown()
                        || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
                    {
                        return Err(Error::new(E_FAIL, "Surface deferred click unauthorized"));
                    }
                    let binding = self.binding().clone();
                    let revision = self.snapshot_revision;
                    let events = self.window.prepare_deferred(&pending, &binding, revision)?;
                    self.refresh(now_ms)?;
                    if self.stopped.is_some()
                        || !self.is_shown()
                        || self.binding() != &binding
                        || self.snapshot_revision != revision
                        || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
                    {
                        return Err(Error::new(E_FAIL, "Surface deferred click became stale"));
                    }
                    return Ok(Some(events));
                }
                FenceState::Pending => {}
            }
        }
        if self.window.has_pending_edit() {
            self.refresh(now_ms)?;
            if self.stopped.is_some()
                || !self.is_shown()
                || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
            {
                return Err(Error::new(E_FAIL, "Surface edit is not authorized"));
            }
            let binding = self.binding().clone();
            let revision = self.snapshot_revision;
            let events = self.window.prepare_edit_before(
                &binding,
                revision,
                self.deferred_click.as_ref(),
            )?;
            let refreshed = self.refresh(now_ms);
            if refreshed.is_err()
                || self.stopped.is_some()
                || !self.is_shown()
                || self.binding() != &binding
                || self.snapshot_revision != revision
                || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
            {
                self.window.cancel_edits(&events);
                refreshed?;
                return Err(Error::new(E_FAIL, "Surface edit became stale"));
            }
            if events.is_empty()
                && self.deferred_click.as_ref().is_some_and(|pending| {
                    self.window.deferred_state(pending)
                        == super::surface_edit_mount::FenceState::Ready
                })
            {
                return self.take_prepared_click(now_ms);
            }
            return Ok(Some(events));
        }
        let Some(click) = self.window.take_click() else {
            return Ok(None);
        };
        self.prepare_click(click, now_ms).map(Some)
    }
    pub fn cancel_prepared_input(&mut self, events: Vec<crate::surface_click_event::ClickEvent>) {
        if let Some(js) = self.javascript.as_mut() {
            // 未移交 owner/未尝试的完整 emit 保留原 ID/字节，不重新执行 listener。
            if js.events.is_none() && events.iter().all(|event| !event.attempted()) {
                js.events = Some(events);
            }
        } else {
            self.window.cancel_edits(&events);
        }
    }
    /// 输入任务只从当前已显示、仍获授权且版本匹配的窗口取得唯一 owner。
    pub fn begin_event(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
        now_ms: u64,
    ) -> Result<Option<Owner>> {
        self.refresh(now_ms)?;
        if self.javascript.is_some()
            || self.stopped.is_some()
            || !self.is_shown()
            || event.revision() != self.snapshot_revision
        {
            return Ok(None);
        }
        Ok(self.lifecycle.take_event(event))
    }
    /// 整批从当前呈现窗口取得唯一 owner，预检版本/身份/容量后才转移。
    pub fn begin_batch(
        &mut self,
        events: &[crate::surface_click_event::ClickEvent],
        now_ms: u64,
    ) -> Result<Option<Owner>> {
        if let Err(error) = self.refresh(now_ms) {
            self.window.cancel_edits(events);
            return Err(error);
        }
        if !self.javascript_ready()
            || self.stopped.is_some()
            || !self.is_shown()
            || events
                .iter()
                .any(|event| event.is_javascript() != self.javascript.is_some())
        {
            self.window.cancel_edits(events);
            return Ok(None);
        }
        if let Err(error) = self.window.set_submission_enabled(false) {
            self.window.cancel_edits(events);
            return Err(error);
        }
        if self.javascript.is_some() {
            self.validate_javascript_input_viewport()?;
        }
        let owner = self.lifecycle.take_batch(events, self.snapshot_revision);
        if owner.is_none() {
            self.window.cancel_edits(events);
        }
        Ok(owner)
    }
    /// 仅恢复批次 owner，不接纳任何场景；全部结果仍由调用方负责保存。
    pub fn finish_batch(
        &mut self,
        events: &[crate::surface_click_event::ClickEvent],
        owner: Owner,
    ) -> std::result::Result<(), Box<Owner>> {
        self.lifecycle.restore_batch(events, owner)
    }
    /// 仅返还 owner；ACK 和交付资格仍由调用方保留，恢复 owner 不重新显示窗口。
    pub fn finish_event(
        &mut self,
        event: &crate::surface_click_event::ClickEvent,
        owner: Owner,
    ) -> std::result::Result<(), Box<Owner>> {
        self.lifecycle.restore_event(event, owner)
    }
    pub fn begin_activation(&mut self, now_ms: u64) -> Result<Option<Owner>> {
        self.refresh(now_ms)?;
        if self.stopped.is_some() || !self.is_alive() || !self.javascript_ready() {
            return Ok(None);
        }
        Ok(self.lifecycle.take())
    }
    pub fn finish_activation(&mut self, owner: Owner) -> std::result::Result<(), Box<Owner>> {
        self.lifecycle.restore(owner)
    }
    pub fn begin_deactivation(&mut self, now_ms: u64) -> Result<Option<Owner>> {
        self.refresh(now_ms)?;
        if self.is_alive() || self.stopped.is_some() || !self.javascript_finished() {
            return Ok(None);
        }
        Ok(self.lifecycle.take_inactive())
    }
    pub fn activation_error(&self) -> Option<&str> {
        self.activation_error.as_deref()
    }
    pub(super) fn failed_start(&mut self, error: crate::surface_lifecycle_runtime::StartError) {
        self.activation_error = Some(error.error.to_string());
        // StartError 合同返还原 owner；即使合同被破坏也保留拒收的证据。
        if let Err(owner) = self.finish_activation(*error.owner) {
            self.rejected_owner = Some(owner);
        }
    }
    pub fn rejected_owner(&self) -> Option<&Owner> {
        self.rejected_owner.as_deref()
    }
    pub(super) fn record_activation_error(&mut self, error: String) {
        self.activation_error = Some(error);
    }
    pub fn request(&self) -> &crate::surface_attach_runtime::Request {
        &self.request
    }
    pub fn stopped(&self) -> Option<LeaseError> {
        self.stopped
    }
    /// 应用等待集合必须登记此句柄，并在 expires_at_ms 到期时结束等待。
    pub fn revocation_handle(&self) -> BorrowedHandle<'_> {
        self.notice.handle()
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.images
            .borrow()
            .values()
            .map(|image| image.request.expires_at_ms())
            .min()
            .unwrap_or(u64::MAX)
            .min(self.lease.expires_at_ms())
            .min(self.javascript_deadline())
    }
    /// 在应用消息结算点及显示前调用；本地关闭后仍保留附件绑定和版本。
    pub fn refresh(&mut self, now_ms: u64) -> Result<()> {
        if let Err(error) = self
            .lease
            .validate(self.lifecycle.binding().origin(), now_ms)
        {
            self.stopped = Some(error);
            self.resources.get_mut().take();
            self.images.get_mut().clear();
            *self.image_requests.get_mut() = Default::default();
            self.window.close()?;
        }
        if !self.window.is_alive() {
            self.stop_javascript();
            self.resources.get_mut().take();
            self.images.get_mut().clear();
            *self.image_requests.get_mut() = Default::default();
        } else if self.stopped.is_none() {
            if self.javascript.is_none() {
                self.refresh_images(now_ms)?;
            }
            // 到期投影可能调用 HWND；同步重入之后必须再次检查撤权。
            if let Err(error) = self.lease.validate(self.binding().origin(), now_ms) {
                self.stopped = Some(error);
                self.close()?;
            }
        }
        self.poll_javascript(now_ms)
    }
    pub fn show(&mut self, now_ms: u64) -> Result<()> {
        self.refresh(now_ms)?;
        if !self.lifecycle.owner().is_some_and(Owner::confirmed_active) {
            return Err(Error::new(E_FAIL, "Surface activation is not confirmed"));
        }
        if self.stopped.is_some() || !self.window.is_alive() {
            return Err(Error::new(E_FAIL, "Surface authorization ended"));
        }
        // 等待后台同步 Active ACK，不把尚未 ready 记录为永久激活错误。
        if !self.javascript_ready() {
            return Ok(());
        }
        if !self.shown {
            // 激活 ACK 会推进 binding 的生命周期版本；不能沿用 mounted 阶段的绑定。
            self.window.bind_edits(self.binding())?;
            self.window.show()?;
            self.shown = true;
        }
        self.refresh(now_ms)?;
        if self.stopped.is_some() {
            return Err(Error::new(E_FAIL, "Surface authorization ended"));
        }
        Ok(())
    }
    pub fn close(&self) -> Result<()> {
        self.stop_javascript();
        self.resources.borrow_mut().take();
        self.images.borrow_mut().clear();
        *self.image_requests.borrow_mut() = Default::default();
        self.window.close()
    }
    pub fn take_error(&self) -> Option<Error> {
        self.window.take_error()
    }
    /// 关闭本地窗口后把生命周期 owner 和租约交给串行后台事务；不发送隐式请求。
    pub fn into_lifecycle(self) -> (Option<Owner>, Lease) {
        let Self {
            window,
            lease,
            lifecycle,
            ..
        } = self;
        drop(window);
        (lifecycle.into_owner(), lease)
    }
}
