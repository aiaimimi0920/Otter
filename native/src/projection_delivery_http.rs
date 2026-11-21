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
    fn exchange_projection_until(
        &self,
        path: &'static str,
        body: &[u8],
        deadline: Instant,
        limit: usize,
    ) -> Result<(u16, Zeroizing<Vec<u8>>), Error> {
        let shared = matches!(
            path,
            "/v1/projections/targets"
                | "/v1/projections/inbox"
                | "/v1/projections/read"
                | "/v1/projections/create"
                | "/v1/projections/accept"
        );
        let mut delays = [150, 450].into_iter();
        loop {
            let (status, bytes) = self.exchange_until("POST", path, body, deadline, limit, None)?;
            if shared && status == 429 && error_code(&bytes, "projection_busy") {
                // 共用 Loom 在进入 handler 前拒绝这次请求。互联 Loom 的错误没有这个保证。
                let Some(delay) = delays.next().map(Duration::from_millis) else {
                    return Err(Error::ProjectionBusy);
                };
                let cancelled = || {
                    self.cancel
                        .flag()
                        .load(std::sync::atomic::Ordering::Acquire)
                };
                if cancelled() || deadline.saturating_duration_since(Instant::now()) <= delay {
                    return Err(Error::ProjectionBusy);
                }
                drop(bytes);
                std::thread::sleep(delay);
                if cancelled() || Instant::now() >= deadline {
                    return Err(Error::ProjectionBusy);
                }
                continue;
            }
            if path == "/v1/projections/create"
                && status == 409
                && error_code(&bytes, "projection_target_offline")
            {
                return Err(Error::ProjectionTargetOffline);
            }
            return Ok((status, bytes));
        }
    }
    fn send_projection_limited(
        &self,
        path: &'static str,
        body: &[u8],
        limit: usize,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        let (status, bytes) = self.exchange_projection_until(
            path,
            body,
            Instant::now() + Duration::from_secs(12),
            limit,
        )?;
        self.validated_response(status, bytes)
    }
    pub(crate) fn inspect_projection(
        &self,
        request: &delivery::share::Import,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Invitation, Error> {
        let envelope = &request.envelope;
        let body = serde_json::to_vec(
            &serde_json::json!({"sourceLoomId":request.source_loom_id,"envelope":envelope}),
        )
        .map_err(|_| Error::Input)?;
        let mut budget = crate::pixels::reserve(delivery::accept::MAX_RESPONSE.next_power_of_two())
            .ok_or(Error::Projection(delivery::Error::Budget))?;
        let bytes = self.send_limited(
            "POST",
            "/v1/offline-projections/import",
            &body,
            Duration::from_secs(12),
            delivery::accept::MAX_RESPONSE,
        )?;
        if !budget.grow_to(bytes.capacity()) {
            return Err(Error::Projection(delivery::Error::Budget));
        }
        delivery::share::inspected_from(
            &bytes,
            envelope,
            self.client.origin(),
            self.session.device_id(),
            Some(&request.source_loom_id),
            cancel,
        )
        .map_err(Error::Projection)
    }
    /// 只读发送目录；不改变本机接收策略，不依据响应中的地址连接其他来源。
    pub fn projection_targets(&self) -> Result<delivery::targets::Directory, Error> {
        let bytes =
            self.send_projection_limited("/v1/projections/targets", b"{}", delivery::MAX_JSON)?;
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
        path: &'static str,
        body: &[u8],
        invitation: &Invitation,
    ) -> Result<delivery::accept::Received, Error> {
        // read 与 accept 共用相同压缩图/共享预算，不扩大收件元数据限制。
        let mut wire_budget =
            crate::pixels::reserve(delivery::accept::MAX_RESPONSE.next_power_of_two())
                .ok_or(Error::Projection(delivery::Error::Budget))?;
        let bytes = self.send_projection_limited(path, body, delivery::accept::MAX_RESPONSE)?;
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
        let bytes =
            self.send_projection_limited("/v1/projections/inbox", &body, delivery::MAX_JSON)?;
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

fn error_code(bytes: &[u8], expected: &str) -> bool {
    #[derive(serde::Deserialize)]
    struct Failure<'a> {
        #[serde(borrow)]
        error: Code<'a>,
    }
    #[derive(serde::Deserialize)]
    struct Code<'a> {
        #[serde(borrow)]
        code: &'a str,
    }
    bytes.len() <= delivery::MAX_JSON
        && serde_json::from_slice::<Failure<'_>>(bytes)
            .is_ok_and(|failure| failure.error.code == expected)
}
