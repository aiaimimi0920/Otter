//! 删除只将原意图单向提升为待清理；仍须原设备认证和 unlinked:true 才能退休。
use super::*;

impl Ticket {
    pub(crate) fn cleanup_pending(&self) -> bool {
        self.record.cleanup
    }
    pub(crate) fn same_intent(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.store.0, &other.store.0) && self.record.same_intent(&other.record)
    }
    /// worker/终态回收调用；旧快照仅能减权标记，不能恢复接收或跨 Store/身份换意图。
    pub(crate) fn queue_cleanup(mut self) -> Result<Self, Error> {
        {
            let state = self.store.0.state.lock().map_err(|_| Error::Changed)?;
            self.record = state
                .entries
                .iter()
                .find(|current| current.same_intent(&self.record))
                .ok_or(Error::Changed)?
                .clone();
        }
        self.activate()?; // 复用同 intent 串行许可和完整磁盘 CAS；活动接收不能被抢占。
        if !self.record.cleanup {
            let mut state = self.store.0.state.lock().map_err(|_| Error::Changed)?;
            let mut next = state.entries.clone();
            let current = next
                .iter_mut()
                .find(|current| **current == self.record)
                .ok_or(Error::Changed)?;
            current.cleanup = true;
            self.store.persist(&mut state, next)?;
            self.record.cleanup = true;
        }
        Ok(self)
    }
}
