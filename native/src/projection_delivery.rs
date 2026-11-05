//! 传统投送合同；全量校验后发布，不复制凭据，接收响应不等于已经持久化或呈现。
use serde::{Deserialize, Serialize};
#[path = "projection_envelope.rs"]
mod envelope;
pub use envelope::Envelope;
#[cfg(windows)]
#[path = "projection_accept.rs"]
pub mod accept;
#[cfg(windows)]
#[path = "projection_receive_binding.rs"]
pub mod binding;
#[cfg(windows)]
#[path = "projection_receive_journal.rs"]
pub mod journal;
#[path = "projection_receive_route.rs"]
mod receive_route;
#[path = "projection_sender.rs"]
pub mod sender;
#[path = "projection_targets.rs"]
pub mod targets;
pub(crate) use receive_route::Operation as ReceiveOperation;
#[path = "projection_inbox_decode.rs"]
mod inbox_decode;
pub const MAX_JSON: usize = 1024 * 1024;
pub const MAX_INVITATIONS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Limit,
    Json,
    Origin,
    Envelope,
    Entry,
    Duplicate,
    AlreadyAccepted,
    Receipt,
    Response,
    Snapshot,
    Budget,
    Stopped,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InboxPolicy {
    Disabled,
    Confirm,
    Auto,
}
#[derive(Clone, Debug)]
pub struct Invitation {
    envelope: Envelope,
    revision: u64,
    digest: String,
    source_name: String,
    accepted: bool,
    offline: Option<receive_route::Offline>,
}
impl Invitation {
    pub fn envelope(&self) -> &Envelope {
        &self.envelope
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn source_name(&self) -> &str {
        &self.source_name
    }
    pub fn accepted(&self) -> bool {
        self.accepted
    }
    /// 仅拒绝仍待本机同意的新邀请；不能把新的接收策略套在既有 accepted 关联上。
    pub(crate) fn reject_body(&self, origin: &str) -> Result<Vec<u8>, Error> {
        self.validate_for(origin)?;
        if self.accepted {
            return Err(Error::AlreadyAccepted);
        }
        // 拒绝不增加接收权；过期邀请仍可交由服务端记录/清理，不套用 accept 的期限。
        serde_json::to_vec(&serde_json::json!({
            "projectionId":self.envelope.projection_id(), "status":"rejected"
        }))
        .map_err(|_| Error::Json)
    }
}
#[derive(Debug)]
pub struct Inbox {
    entries: Vec<Invitation>,
    offline_peers: bool,
}
impl Inbox {
    pub fn entries(&self) -> &[Invitation] {
        &self.entries
    }
    /// 离线查询未完成时保留标记；成功合并才清除，不把共享结果当完整跨Loom收件箱。
    pub fn has_offline_peers(&self) -> bool {
        self.offline_peers
    }
    pub fn into_entries(self) -> Vec<Invitation> {
        self.entries
    }
}
pub(crate) fn receipt(bytes: &[u8]) -> Result<(), Error> {
    if bytes.len() > MAX_JSON {
        return Err(Error::Limit);
    }
    #[derive(Deserialize)]
    struct Receipt {
        recorded: bool,
    }
    let receipt: Receipt = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
    if receipt.recorded {
        Ok(())
    } else {
        Err(Error::Receipt)
    }
}
/// 解绑只减权；仍严格绑定原 HTTPS 来源，由协议 owner 统一验证 envelope。
pub(crate) fn unlink_body(envelope: &Envelope, origin: &str) -> Result<Vec<u8>, Error> {
    envelope.validate(origin)?;
    serde_json::to_vec(&serde_json::json!({"projectionId": envelope.projection_id()}))
        .map_err(|_| Error::Json)
}
#[cfg(windows)]
pub(crate) fn displayed_body(
    intent: &journal::Ticket,
    origin: &str,
    device: &str,
) -> Result<Vec<u8>, Error> {
    intent.invitation().validate_for(origin)?;
    if intent.device() != device {
        return Err(Error::Response);
    }
    serde_json::to_vec(&serde_json::json!({
        "projectionId":intent.envelope().projection_id(), "status":"displayed"
    }))
    .map_err(|_| Error::Json)
}
#[cfg(test)]
#[path = "projection_delivery_tests.rs"]
pub(crate) mod tests;
