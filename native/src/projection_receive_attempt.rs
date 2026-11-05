//! 原接收 Job 的精确票据交接；原活动许可留至 join，失败不扫描或猜测 journal。
use super::Error;
use crate::projection_delivery::{
    journal::{Store, Ticket},
    Invitation,
};
use std::sync::Mutex;

#[derive(Default)]
pub(super) struct Attempt(Mutex<Option<(Ticket, bool)>>);
impl Attempt {
    pub(super) fn begin(
        &self,
        store: &Store,
        invitation: &Invitation,
        device: &str,
    ) -> Result<Ticket, Error> {
        let (ticket, created) = store
            .begin_attempt(invitation, device)
            .map_err(Error::Journal)?;
        self.activate(ticket, created)
    }
    pub(super) fn recover(&self, ticket: Ticket) -> Result<Ticket, Error> {
        // 只读恢复不是新 Accept；失败/panic 仍保原许可，但永不产生自动补偿资格。
        self.activate(ticket, false)
    }
    pub(super) fn reattach(
        &self,
        store: &Store,
        invitation: &Invitation,
        device: &str,
    ) -> Result<Ticket, Error> {
        if !invitation.accepted() {
            return Err(Error::Stopped);
        }
        let ticket = store.begin(invitation, device).map_err(Error::Journal)?;
        // 本地新记录不等于远端新关联；失败/取消只能保留，绝不能自动解绑旧 receiver。
        self.activate(ticket, false)
    }
    fn activate(&self, mut ticket: Ticket, created: bool) -> Result<Ticket, Error> {
        // 只有成功 activate 才可能发网；Busy/CAS 失败不抢占另一个接收的补偿责任。
        ticket.activate().map_err(Error::Journal)?;
        let snapshot = ticket.retry();
        *self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((ticket, created));
        Ok(snapshot)
    }
    pub(super) fn take(&self) -> Option<(Ticket, bool)> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
    pub(super) fn abandon(&self) -> Option<Ticket> {
        let (ticket, created) = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()?;
        // marker 的 retry 需重新 activate：返回无许可快照并释放已回收网络的原许可。
        // 复用旧 intent 无稳定关联排除证明，只保原 journal 的显式恢复责任。
        created.then(|| ticket.retry())
    }
}
