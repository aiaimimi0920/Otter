//! 同 ID read→明确不存在→Create；丢失回复只再读一次，不重发 Create、不换签名身份。
use super::*;
use delivery::sender::{Active, Operation, Response};
#[path = "projection_sender_update_http.rs"]
mod update;
impl Control<'_> {
    pub(crate) fn send_projection(&self, active: &Active) -> Result<Response, Error> {
        if let Some(response) = self.read_sender(active)? {
            return Ok(response);
        }
        let body = active.create_body().map_err(Error::Sender)?;
        let result = self
            .sender_exchange(active, active.intent().path(Operation::Create), &body, true)
            .and_then(|response| response.ok_or(Error::Input));
        match result {
            Ok(response) => Ok(response),
            Err(original) => {
                // 包括拒绝/坏响应/不确定 HTTP；一次只读 reconcile 不会增加远端副作用。
                active.check().map_err(Error::Sender)?;
                match self.read_sender(active) {
                    Ok(Some(response)) => Ok(response),
                    _ => Err(original),
                }
            }
        }
    }
    fn read_sender(&self, active: &Active) -> Result<Option<Response>, Error> {
        let body = active.read_body().map_err(Error::Sender)?;
        self.sender_exchange(active, active.intent().path(Operation::Read), &body, false)
    }
    fn sender_exchange(
        &self,
        active: &Active,
        path: &'static str,
        body: &[u8],
        created: bool,
    ) -> Result<Option<Response>, Error> {
        active.check().map_err(Error::Sender)?;
        if active.intent().envelope().origin() != self.client.origin()
            || active.intent().envelope().source_device() != self.session.device_id()
        {
            return Err(Error::Input);
        }
        let limit = delivery::accept::MAX_RESPONSE;
        let mut budget = crate::pixels::reserve(limit.next_power_of_two())
            .ok_or(Error::Sender(delivery::sender::Error::Limit))?;
        let (status, bytes) = self.exchange_until(
            "POST",
            path,
            body,
            Instant::now() + Duration::from_secs(12),
            limit,
            None,
        )?;
        if !budget.grow_to(bytes.capacity()) {
            return Err(Error::Sender(delivery::sender::Error::Limit));
        }
        active.check().map_err(Error::Sender)?;
        self.session
            .token_for(self.client.origin(), now()?)
            .map_err(|_| Error::Credential)?;
        if status == 401 {
            return Err(Error::AuthorizationRequired);
        }
        if !(200..300).contains(&status) {
            // 普通 404、网关正文、其他业务码与任何其他 HTTP 状态都不授权创建。
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
            if !created
                && status == 404
                && bytes.len() <= delivery::MAX_JSON
                && serde_json::from_slice::<Failure<'_>>(&bytes)
                    .is_ok_and(|error| error.error.code == "projection_not_found")
            {
                return Ok(None);
            }
            return Err(Error::Status(status));
        }
        Response::parse(
            &bytes,
            active.intent(),
            created,
            active.lease().preparation_cancel(),
        )
        .map(Some)
        .map_err(Error::Sender)
    }
    pub(crate) fn unlink_sender(&self, active: &Active) -> Result<(), Error> {
        active.check().map_err(Error::Sender)?;
        if active.intent().envelope().source_device() != self.session.device_id() {
            return Err(Error::Input);
        }
        self.unlink_projection_at(
            active.intent().envelope(),
            active.intent().path(Operation::Unlink),
        )?;
        active.retire().map_err(Error::Sender)
    }
}
