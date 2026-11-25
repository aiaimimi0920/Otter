//! Traditional-delivery control on shared Loom uses Device/independent nonce, fixed origin, cancellation, and bounded HTTPS.
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
                // Shared Loom rejected this request before entering the handler; interconnected-Loom errors provide no such guarantee.
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
    /// Read-only sender catalogs; do not change local receive policies or connect elsewhere based on response addresses.
    pub fn projection_targets(&self) -> Result<delivery::targets::Directory, Error> {
        let bytes =
            self.send_projection_limited("/v1/projections/targets", b"{}", delivery::MAX_JSON)?;
        delivery::targets::Directory::parse(&bytes, self.client.origin()).map_err(Error::Projection)
    }
    /// Compensation requests unlink only at the original origin; non-2xx or absent unlinked:true cannot retire write-ahead intent.
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
    /// Success means only server admission and correlated compressed PNG receipt; never send displayed or create windows in the network layer.
    pub fn accept_projection(
        &self,
        invitation: &Invitation,
    ) -> Result<delivery::accept::Received, Error> {
        let body = invitation
            .accept_body(self.client.origin())
            .map_err(Error::Projection)?;
        self.projection_snapshot(invitation.path(ReceiveOperation::Accept), &body, invitation)
    }
    /// Recovery reads existing receiver associations only; knownRevision=0 requests the current full image without accepting again.
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
        // read and accept share compressed-image/shared-budget limits, without enlarging inbox-metadata limits.
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
    /// Callers explicitly select participation policy; no automatic polling, receive enablement, invitation handling, or display claims.
    pub fn projection_inbox(&self, policy: InboxPolicy) -> Result<Inbox, Error> {
        let body = serde_json::to_vec(&serde_json::json!({"policy":policy}))
            .map_err(|_| Error::Projection(delivery::Error::Json))?;
        let bytes =
            self.send_projection_limited("/v1/projections/inbox", &body, delivery::MAX_JSON)?;
        let mut inbox = Inbox::parse(&bytes, self.client.origin()).map_err(Error::Projection)?;
        drop(bytes); // Release original wire before the second bounded inbox response; never retain two raw JSON bodies together.
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
                Err(_) => {} // Retain shared results/unqueried markers; never reuse old offline candidates or truncate oversized lists.
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
    /// Only original receiver owners submit after saving/actual presentation; menus cannot select arbitrary receipt states.
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
    /// rejected remains limited to unaccepted invitations; do not share eligibility with post-accept displayed.
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
