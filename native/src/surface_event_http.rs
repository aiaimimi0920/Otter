//! 单次 Surface click 提交；仅后台调用，错误不触发重放或管理员凭据回退。
use super::*;
use crate::{
    surface_action_ack::{Ack, MAX_ACK_BYTES},
    surface_click_event::ClickEvent,
    surface_lifecycle_owner::Owner,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitError {
    AlreadyAttempted,
    Inactive,
    Stale,
    Identity,
    Transport(Error),
    Ack(crate::surface_action_ack::Error),
}
impl Control<'_> {
    /// current_revision 必须由当前呈现 owner 提供，不能用 event 自带版本冒充新检查。
    /// attempted=true 后没有匹配 ACK 即为未确认，调用方保留 event 供后续查询收敛。
    pub fn submit_surface_click(
        &self,
        owner: &Owner,
        current_revision: u64,
        event: &mut ClickEvent,
    ) -> Result<Ack, SubmitError> {
        self.submit_surface_click_with_timeout(
            owner,
            current_revision,
            event,
            Duration::from_secs(10),
        )
    }
    pub(super) fn submit_surface_click_with_timeout(
        &self,
        owner: &Owner,
        current_revision: u64,
        event: &mut ClickEvent,
        timeout: Duration,
    ) -> Result<Ack, SubmitError> {
        if event.attempted() {
            return Err(SubmitError::AlreadyAttempted);
        }
        if !owner.confirmed_active() {
            return Err(SubmitError::Inactive);
        }
        if owner.binding() != event.binding() || current_revision != event.revision() {
            return Err(SubmitError::Stale);
        }
        let binding = owner.binding();
        if binding.origin() != self.client.origin()
            || binding.device() != self.session.device_id()
            || matches!(binding.instance(), "." | "..")
        {
            return Err(SubmitError::Identity);
        }
        let mut url =
            url::Url::parse("https://path.invalid/").map_err(|_| SubmitError::Identity)?;
        url.path_segments_mut()
            .map_err(|_| SubmitError::Identity)?
            .clear()
            .extend(["v1", "surfaces", "instances", binding.instance(), "events"]);
        // 从此处开始保守禁止再次提交，即使认证/网络层未确认送达，也不猜测服务端效果。
        event.begin_submission();
        let bytes = self
            .send_limited("POST", url.path(), event.bytes(), timeout, MAX_ACK_BYTES)
            .map_err(SubmitError::Transport)?;
        Ack::parse(&bytes, event).map_err(SubmitError::Ack)
    }
}
