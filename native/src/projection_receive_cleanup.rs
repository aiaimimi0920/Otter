//! Deletion only moves original intent one-way to pending cleanup; retirement still requires original device authentication and unlinked:true.
use super::*;

impl Ticket {
    pub(crate) fn cleanup_pending(&self) -> bool {
        self.record.cleanup
    }
    pub(crate) fn same_intent(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.store.0, &other.store.0) && self.record.same_intent(&other.record)
    }
    /// Worker/terminal-reclamation calls; old snapshots may only mark privilege reduction, not restore receiving or switch intent across Store/identity.
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
        self.activate()?; // Reuse same-intent serial permits/full disk CAS; active receives cannot be preempted.
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
