//! 明确服务端拒绝先持久化；只读对账和Checkpoint均完成前保留原请求，不把未知失败当拒绝。
use super::*;
impl Ticket {
    pub(crate) fn rejection(&self) -> Option<super::super::Code> {
        self.pending_record().rejected
    }
    pub(crate) fn reject(
        &self,
        code: super::super::Code,
        cancel: &AtomicBool,
    ) -> Result<Self, Error> {
        if !code.recoverable()
            || !matches!(
                self.request().operation(),
                super::super::Operation::Apply { .. }
                    | super::super::Operation::Mode { .. }
                    | super::super::Operation::Checkpoint { .. }
            )
            || self.previous().is_none()
        {
            return Err(Error::Format);
        }
        let mut state = self.store.0.state.lock().map_err(|_| Error::Changed)?;
        self.store.baseline(&state, cancel)?;
        let mut entries = state.entries.clone();
        let old = entries
            .iter_mut()
            .find(|e| **e == self.entry)
            .ok_or(Error::Changed)?;
        old.pending.as_mut().ok_or(Error::Changed)?.rejected = Some(code);
        let entry = old.clone();
        self.store.persist(&mut state, entries, cancel)?;
        Ok(Self {
            store: self.store.clone(),
            entry,
        })
    }
    /// 仅调用者真实默认Checkpoint包含拒绝后的权威/可见冲突时调用；read本身不授ack。
    pub(crate) fn commit_rejected_after_checkpoint(
        &self,
        document: Document,
        cancel: &AtomicBool,
    ) -> Result<(), Error> {
        if self.rejection().is_none() {
            return Err(Error::Changed);
        }
        document
            .follows(self.previous().ok_or(Error::Changed)?)
            .map_err(|_| Error::Changed)?;
        let mut state = self.store.0.state.lock().map_err(|_| Error::Changed)?;
        self.store.baseline(&state, cancel)?;
        let mut entries = state.entries.clone();
        let old = entries
            .iter_mut()
            .find(|e| **e == self.entry)
            .ok_or(Error::Changed)?;
        old.document = Some(document);
        old.pending = None;
        self.store.persist(&mut state, entries, cancel)
    }
}
#[cfg(test)]
#[path = "projection_edit_journal_rejection_tests.rs"]
mod tests;
