//! Original Store owns stable-unlink persistent privilege reduction/success receipts; receipts grant no network capability, and ACK accepts only exact canonical proof.
use super::*;
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "projection_sender_stop_tests.rs"]
mod tests;
impl Request {
    pub(crate) fn stop_requested(&self) -> bool {
        self.intent().store.0.state.lock().is_ok_and(|state| {
            state
                .entries
                .iter()
                .any(|entry| entry.same_intent(&self.intent().record) && entry.stop.is_some())
        })
    }
    pub(crate) fn published_unlinked(&self) -> bool {
        self.intent().store.0.state.lock().is_ok_and(|state| {
            state.entries.iter().any(|entry| {
                entry.same_intent(&self.intent().record)
                    && entry.stop == Some(record::Stop::Unlinked)
            })
        })
    }
    /// Explicit background user intent; two revoke calls close activate-registration races, never implicitly invoked by ordinary exit.
    pub(crate) fn request_stop(&self) -> Result<(), Error> {
        let intent = &self.intent();
        let store = &intent.store;
        store.0.activity.stop(&intent.record);
        let cancel = AtomicBool::new(false);
        let mut state = store.0.state.lock().map_err(|_| Error::Changed)?;
        store.0.activity.stop(&intent.record);
        store.baseline(&state, &cancel)?;
        let index = state
            .entries
            .iter()
            .position(|entry| entry.same_intent(&intent.record))
            .ok_or(Error::Changed)?;
        if state.entries[index].stop.is_some() {
            return Ok(());
        }
        let mut next = state.entries.clone();
        next[index].cancelled = true;
        next[index].stop = Some(record::Stop::Requested);
        store.persist(&mut state, next, &cancel)
    }
    /// Only canonical commit owners construct proof; background work still rechecks full journal baselines/original success receipts.
    pub(crate) fn acknowledge_stop(
        &self,
        proof: &binding::stop::Acknowledgement,
    ) -> Result<(), Error> {
        if !proof.matches(self.intent()) {
            return Err(Error::Changed);
        }
        let intent = &self.intent();
        let store = &intent.store;
        let cancel = AtomicBool::new(false);
        let mut state = store.0.state.lock().map_err(|_| Error::Changed)?;
        store.baseline(&state, &cancel)?;
        if !state.entries.iter().any(|entry| {
            entry.same_intent(&intent.record) && entry.stop == Some(record::Stop::Unlinked)
        }) {
            return Err(Error::Changed);
        }
        let next = state
            .entries
            .iter()
            .filter(|entry| !entry.same_intent(&intent.record))
            .cloned()
            .collect();
        store.persist(&mut state, next, &cancel)
    }
}
