//! 发送结果保原四槽/租约/同意图活动许可，工作区持久绑定前不退休原写前记录。
use super::*;
use crate::projection_delivery::sender::{Active, Request, Response};
pub(super) struct Connected {
    pub active: Active,
    pub response: Response,
    pub candidate: Option<crate::projection_delivery::sender::binding::Candidate>,
}
pub struct Ready {
    pub(super) value: Connected,
    _slot: Slot,
}
impl Ready {
    pub(super) fn new(value: Connected, slot: Slot) -> Self {
        Self { value, _slot: slot }
    }
    pub fn response(&self) -> &Response {
        &self.value.response
    }
    pub fn request(&self) -> Request {
        self.value.active.request().clone()
    }
    pub fn validate(&self, current: &Lease, now: u64) -> Result<&Response, Error> {
        let active = &self.value.active;
        active.check().map_err(Error::Sender)?;
        current
            .validate(active.intent().envelope().origin(), now)
            .map_err(|_| Error::Stopped)?;
        if !active.lease().same_grant(current) {
            return Err(Error::Stopped);
        }
        Ok(&self.value.response)
    }
    /// 后台调用：完整 outbox 基线和原会话 patch 均可能有大 I/O/编码，不能在 UI 菜单执行。
    /// 返回的候选保原日志；调用方仍须复核源/工作区/授权，并等自己的精确 checkpoint 提交。
    pub fn prepare_binding(
        &self,
        source: &crate::session_source::Source,
        name: &str,
        current: &Lease,
        now: u64,
    ) -> Result<crate::projection_delivery::sender::binding::Candidate, Error> {
        let response = self.validate(current, now)?;
        let candidate = crate::projection_delivery::sender::binding::Binding::prepare(
            source,
            self.request().intent(),
            response,
            name,
            current.preparation_cancel(),
        )
        .map_err(Error::Sender)?;
        self.validate(current, now)?;
        Ok(candidate)
    }
    pub fn request_stop(&self) {
        self.value.active.lease().revoke();
    }
}
impl std::fmt::Debug for Ready {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.response.fmt(f)
    }
}
