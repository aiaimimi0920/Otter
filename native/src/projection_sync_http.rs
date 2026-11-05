//! 持续 read 复用固定 origin/Device nonce/取消/原响应预算；只识别封闭终态错误码。
use super::*;
use delivery::{accept::Read, binding::Binding, journal::Ticket};
impl Control<'_> {
    pub(crate) fn sync_projection(
        &self,
        ticket: &Ticket,
        expected: &Binding,
    ) -> Result<Read, Error> {
        if ticket.origin() != self.client.origin()
            || !expected.matches_ticket(ticket)
            || ticket.device() != self.session.device_id()
            || expected.stopped()
            || ticket.cleanup_pending()
        {
            return Err(Error::Input);
        }
        let body = serde_json::to_vec(&serde_json::json!({
            "projectionId":ticket.envelope().projection_id(), "knownRevision":expected.revision()
        }))
        .map_err(|_| Error::Input)?;
        let mut budget = crate::pixels::reserve(delivery::accept::MAX_RESPONSE.next_power_of_two())
            .ok_or(Error::Projection(delivery::Error::Budget))?;
        let (status, bytes) = self.exchange_until(
            "POST",
            ticket.path(delivery::ReceiveOperation::Read),
            &body,
            Instant::now() + Duration::from_secs(12),
            delivery::accept::MAX_RESPONSE,
            None,
        )?;
        if !budget.grow_to(bytes.capacity()) {
            return Err(Error::Projection(delivery::Error::Budget));
        }
        // 401 是授权问题，不把不相关 HTTP 状态推断成关联已终止。
        if status == 401 {
            return Err(Error::AuthorizationRequired);
        }
        if !(200..300).contains(&status) {
            #[derive(serde::Deserialize)]
            struct Failure {
                error: FailureCode,
            }
            #[derive(serde::Deserialize)]
            struct FailureCode {
                code: delivery::binding::StopReason,
            }
            if bytes.len() <= delivery::MAX_JSON {
                if let Ok(failure) = serde_json::from_slice::<Failure>(&bytes) {
                    return Err(Error::ProjectionStopped(failure.error.code));
                }
            }
            return Err(Error::Status(status));
        }
        self.session
            .token_for(self.client.origin(), now()?)
            .map_err(|_| Error::Credential)?;
        Read::parse(
            &bytes,
            ticket,
            expected,
            self.client.origin(),
            self.session.device_id(),
        )
        .map_err(Error::Projection)
    }
}
