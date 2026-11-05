//! 固定设备认证edit端点；修改必须持有效写前Ticket，失败/取消不退休pending、不自动重发。
use super::*;
use crate::projection_edit::{
    self as edit,
    journal::{Context, Ticket},
    Code, Document, Failure, Request, Route,
};
impl Control<'_> {
    pub fn read_projection_edit(
        &self,
        context: &Context,
        projection: &str,
        route: Route,
        previous: Option<&Document>,
    ) -> Result<Document, Failure> {
        let request = Request::prepare(edit::Operation::Read {
            projection_id: projection.into(),
        })
        .map_err(Failure::Protocol)?;
        self.exchange_projection_edit(context, route, &request, previous)
    }
    pub fn projection_edit(&self, ticket: &Ticket) -> Result<Document, Failure> {
        if ticket.rejection().is_some() {
            return Err(Failure::Journal(edit::journal::Error::Changed));
        }
        let _active = ticket
            .activate(self.cancel.flag())
            .map_err(Failure::Journal)?;
        let result = self.exchange_projection_edit(
            ticket.context(),
            ticket.route(),
            ticket.request(),
            ticket.previous(),
        );
        ticket.check(self.cancel.flag()).map_err(Failure::Journal)?;
        result
    }
    fn exchange_projection_edit(
        &self,
        context: &Context,
        route: Route,
        request: &Request,
        previous: Option<&Document>,
    ) -> Result<Document, Failure> {
        if context.origin() != self.client.origin() || context.device() != self.session.device_id()
        {
            return Err(Failure::Protocol(edit::Error::Identity));
        }
        if self.cancel.is_stopped() {
            return Err(Failure::Protocol(edit::Error::Stopped));
        }
        let mut budget = crate::pixels::reserve(edit::MAX_BYTES.next_power_of_two())
            .ok_or(Failure::Protocol(edit::Error::Limit))?;
        let (status, bytes) = self
            .exchange_until(
                "POST",
                route.path(),
                request.bytes(),
                Instant::now() + Duration::from_secs(12),
                edit::MAX_BYTES,
                None,
            )
            .map_err(Failure::Transport)?;
        if !budget.grow_to(bytes.capacity()) {
            return Err(Failure::Protocol(edit::Error::Limit));
        }
        if self.cancel.is_stopped() {
            return Err(Failure::Protocol(edit::Error::Stopped));
        }
        self.session
            .token_for(self.client.origin(), now().map_err(Failure::Transport)?)
            .map_err(|_| Failure::Transport(Error::Credential))?;
        if status == 401 {
            return Err(Failure::Transport(Error::AuthorizationRequired));
        }
        if !(200..300).contains(&status) {
            // 只取已知固定错误码，不把远端正文/凭据写入日志，也不跟随重定向。
            let code = edit::json::parse(&bytes, edit::MAX_BYTES)
                .ok()
                .and_then(|v| v["error"]["code"].as_str().map(|s| Code::parse(status, s)))
                .unwrap_or(Code::Other);
            return Err(Failure::Rejected { status, code });
        }
        request
            .response(&bytes, previous)
            .map_err(Failure::Protocol)
    }
}
