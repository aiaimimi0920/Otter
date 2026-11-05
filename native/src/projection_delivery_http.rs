//! 共享 Loom 的传统投送控制；沿用 Device/独立 nonce、固定 origin、取消和有界 HTTPS。
use super::*;
use crate::projection_delivery::{
    self as delivery, Inbox, InboxPolicy, Invitation, ReceiveOperation,
};
#[path = "projection_sender_http.rs"]
mod sender;
#[path = "projection_sync_http.rs"]
mod sync;

impl Control<'_> {
    /// 只读发送目录；不改变本机接收策略，不依据响应中的地址连接其他来源。
    pub fn projection_targets(&self) -> Result<delivery::targets::Directory, Error> {
        let bytes = self.send_limited(
            "POST",
            "/v1/projections/targets",
            b"{}",
            Duration::from_secs(12),
            delivery::MAX_JSON,
        )?;
        delivery::targets::Directory::parse(&bytes, self.client.origin()).map_err(Error::Projection)
    }
    /// 补偿只请求原来源解绑；非 2xx 或缺少 unlinked:true 不能退休写前意图。
    pub fn unlink_projection(&self, envelope: &delivery::Envelope) -> Result<(), Error> {
        self.unlink_projection_at(envelope, "/v1/projections/unlink")
    }
    fn unlink_projection_at(&self, envelope: &delivery::Envelope, path: &str) -> Result<(), Error> {
        let body =
            delivery::unlink_body(envelope, self.client.origin()).map_err(Error::Projection)?;
        self.unlink_projection_body_at(&body, path)
    }
    pub(crate) fn unlink_received_projection(
        &self,
        ticket: &delivery::journal::Ticket,
    ) -> Result<(), Error> {
        ticket
            .invitation()
            .validate_for(self.client.origin())
            .map_err(Error::Projection)?;
        if ticket.device() != self.session.device_id() {
            return Err(Error::Input);
        }
        let body = serde_json::to_vec(
            &serde_json::json!({"projectionId":ticket.envelope().projection_id()}),
        )
        .map_err(|_| Error::Input)?;
        self.unlink_projection_body_at(&body, ticket.path(ReceiveOperation::Unlink))
    }
    fn unlink_projection_body_at(&self, body: &[u8], path: &str) -> Result<(), Error> {
        let bytes = self.send_limited(
            "POST",
            path,
            body,
            Duration::from_secs(12),
            delivery::MAX_JSON,
        )?;
        #[derive(serde::Deserialize)]
        struct Receipt {
            unlinked: bool,
        }
        let receipt: Receipt =
            serde_json::from_slice(&bytes).map_err(|_| Error::Projection(delivery::Error::Json))?;
        if !receipt.unlinked {
            return Err(Error::Projection(delivery::Error::Receipt));
        }
        Ok(())
    }
    /// 成功只代表服务端接入且拿到相关压缩 PNG；不发送 displayed，也不在网络层建窗。
    pub fn accept_projection(
        &self,
        invitation: &Invitation,
    ) -> Result<delivery::accept::Received, Error> {
        let body = invitation
            .accept_body(self.client.origin())
            .map_err(Error::Projection)?;
        self.projection_snapshot(invitation.path(ReceiveOperation::Accept), &body, invitation)
    }
    /// 恢复仅只读现有 receiver 关联；knownRevision=0 要求当前完整图，不重新 accept。
    pub(crate) fn read_projection(
        &self,
        intent: &delivery::journal::Ticket,
    ) -> Result<delivery::accept::Received, Error> {
        if intent.cleanup_pending()
            || intent.device() != self.session.device_id()
            || intent.origin() != self.client.origin()
        {
            return Err(Error::Input);
        }
        let body = serde_json::to_vec(&serde_json::json!({
            "projectionId":intent.envelope().projection_id(), "knownRevision":0
        }))
        .map_err(|_| Error::Projection(delivery::Error::Json))?;
        self.projection_snapshot(
            intent.path(ReceiveOperation::Read),
            &body,
            &intent.invitation(),
        )
    }
    fn projection_snapshot(
        &self,
        path: &str,
        body: &[u8],
        invitation: &Invitation,
    ) -> Result<delivery::accept::Received, Error> {
        // read 与 accept 共用相同压缩图/共享预算，不扩大收件元数据限制。
        let mut wire_budget =
            crate::pixels::reserve(delivery::accept::MAX_RESPONSE.next_power_of_two())
                .ok_or(Error::Projection(delivery::Error::Budget))?;
        let bytes = self.send_limited(
            "POST",
            path,
            body,
            Duration::from_secs(12),
            delivery::accept::MAX_RESPONSE,
        )?;
        if !wire_budget.grow_to(bytes.capacity()) {
            return Err(Error::Projection(delivery::Error::Budget));
        }
        delivery::accept::Received::parse(
            &bytes,
            invitation,
            self.client.origin(),
            self.session.device_id(),
        )
        .map_err(Error::Projection)
    }
    /// 调用方明确选择参与策略；本方法不自动轮询、开启接收、处理邀请或宣称显示。
    pub fn projection_inbox(&self, policy: InboxPolicy) -> Result<Inbox, Error> {
        let body = serde_json::to_vec(&serde_json::json!({"policy":policy}))
            .map_err(|_| Error::Projection(delivery::Error::Json))?;
        let bytes = self.send_limited(
            "POST",
            "/v1/projections/inbox",
            &body,
            Duration::from_secs(12),
            delivery::MAX_JSON,
        )?;
        let mut inbox = Inbox::parse(&bytes, self.client.origin()).map_err(Error::Projection)?;
        drop(bytes); // 第二份有界收件响应前释放原wire，不并存两份原始JSON。
        if inbox.has_offline_peers() && policy != InboxPolicy::Disabled {
            let remote = self
                .send_limited(
                    "POST",
                    "/v1/offline-projections/inbox",
                    &body,
                    Duration::from_secs(12),
                    delivery::MAX_JSON,
                )
                .and_then(|bytes| {
                    Inbox::parse_offline(&bytes, self.client.origin()).map_err(Error::Projection)
                });
            match remote {
                Ok(remote) => {
                    let _ = inbox.merge_offline(remote);
                }
                Err(error @ (Error::AuthorizationRequired | Error::Credential)) => {
                    return Err(error)
                }
                Err(_) => {} // 保留共享结果与未查询标记，绝不复用旧离线候选或截断超限列表。
            }
            if self
                .cancel
                .flag()
                .load(std::sync::atomic::Ordering::Acquire)
            {
                return Err(Error::Http(crate::device_session_client::Error::Stopped));
            }
            self.session
                .token_for(self.client.origin(), now()?)
                .map_err(|_| Error::Credential)?;
        }
        Ok(inbox)
    }
    /// 只供原接收 owner 在保存和真实呈现后提交；不能由菜单选择任意 receipt 状态。
    pub(crate) fn displayed_projection(
        &self,
        intent: &delivery::journal::Ticket,
    ) -> Result<(), Error> {
        let body = delivery::displayed_body(intent, self.client.origin(), self.session.device_id())
            .map_err(Error::Projection)?;
        let bytes = self.send_limited(
            "POST",
            intent.path(ReceiveOperation::Receipt),
            &body,
            Duration::from_secs(12),
            delivery::MAX_JSON,
        )?;
        delivery::receipt(&bytes).map_err(Error::Projection)
    }
    /// rejected 仍只允许未接受邀请，不与接收后的 displayed 混用资格。
    pub fn reject_projection(&self, invitation: &Invitation) -> Result<(), Error> {
        let body = invitation
            .reject_body(self.client.origin())
            .map_err(Error::Projection)?;
        let bytes = self.send_limited(
            "POST",
            invitation.path(ReceiveOperation::Receipt),
            &body,
            Duration::from_secs(12),
            delivery::MAX_JSON,
        )?;
        delivery::receipt(&bytes).map_err(Error::Projection)
    }
}
