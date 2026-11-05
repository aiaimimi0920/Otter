//! 跨后台准备/追加/checkpoint 保留原 Grant、共享容量与同 intent 移动许可；不是落盘或显示证明。
use super::{Error, Lease, Slot};
use crate::projection_delivery::journal::Ticket;
use std::sync::Arc;

pub struct Hold {
    pub(super) lease: Arc<Lease>,
    pub(super) _slot: Slot,
    pub(super) intent: Ticket,
    pub(super) fresh: bool,
}
impl Hold {
    pub fn ticket(&self) -> &Ticket {
        &self.intent
    }
    pub(crate) fn cleanup_ticket(&self) -> Option<Ticket> {
        self.fresh.then(|| self.intent.retry())
    }
    /// 实际保存后放弃自动补偿；索引未知可保守降级，不因此授予 durable 或显示证明。
    pub(crate) fn stabilize(&mut self) {
        self.fresh = false;
    }
    pub fn origin(&self) -> &str {
        self.intent.origin()
    }
    pub fn expires_at_ms(&self) -> u64 {
        self.lease.expires_at_ms()
    }
    pub fn validate(&self, current: &Lease, now: u64) -> Result<(), Error> {
        self.check(now)?;
        current
            .validate(self.origin(), now)
            .map_err(|_| Error::Stopped)?;
        if !self.lease.same_grant(current) {
            return Err(Error::Stopped);
        }
        Ok(())
    }
    pub(crate) fn check(&self, now: u64) -> Result<(), Error> {
        self.lease
            .validate(self.origin(), now)
            .map_err(|_| Error::Stopped)
    }
    pub(crate) fn lease(&self) -> &Arc<Lease> {
        &self.lease
    }
}
