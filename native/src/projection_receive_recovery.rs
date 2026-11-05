//! 显式恢复的只读核验；原 Ticket 许可贯穿 read/像素核验/Ready，不重收或改动本地图。
use super::*;
use crate::projection_delivery::{binding::Binding, journal::Ticket};

pub(super) struct Request {
    pub ticket: Ticket,
    pub expected: Binding,
    pub image: crate::image_decode::Image,
}
impl Request {
    pub(super) fn run(
        self,
        control: &crate::live_relay_http::Control<'_>,
        client: &Client,
        lease: &Lease,
        attempt: &attempt::Attempt,
    ) -> Result<Completed, Error> {
        if self.ticket.cleanup_pending()
            || self.ticket.device() != lease.device_id()
            || self.ticket.origin() != client.origin()
            || !self.expected.matches_ticket(&self.ticket)
        {
            return Err(Error::Stopped);
        }
        // 原 Job 保管带许可 Ticket，worker 只用无许可快照，错误/展开也不提前释放。
        let ticket = attempt.recover(self.ticket)?;
        let response = control.read_projection(&ticket).map_err(Error::Http)?;
        let value = received::decode(response, lease, &ticket)?;
        // PNG 会被普通保存重新编码；比较已解码原像素而非保存后的压缩字节摘要。
        if !crate::projection_delivery::binding::Binding::new(&value, &ticket)
            .map_err(Error::Decode)?
            .same_snapshot(&self.expected)
            || value.image().width != self.image.width
            || value.image().height != self.image.height
            || value.image().pixels != self.image.pixels
            || lease.is_revoked()
        {
            return Err(Error::Stopped);
        }
        Ok(Completed::Accepted(Box::new(value)))
    }
}
