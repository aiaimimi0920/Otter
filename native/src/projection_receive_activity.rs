//! Same-intent network/Ready/consumption ownership; no Mutex across HTTP, and Drop returns only memory permits without disk I/O.
use super::{Error, Store, Ticket};
use std::{
    collections::BTreeSet,
    sync::{atomic::AtomicBool, Mutex},
};
#[derive(Default)]
pub(super) struct Registry(Mutex<BTreeSet<String>>);
pub(super) struct Permit {
    store: Store,
    intent: String,
}
impl Ticket {
    /// Workers claim before networking; permits span Ready -> consumers, preventing old/duplicate Tickets from parallel unlink/rereceive.
    pub(crate) fn activate(&mut self) -> Result<(), Error> {
        if self.permit.is_some() {
            return Ok(());
        }
        let state = self.store.0.state.lock().map_err(|_| Error::Changed)?;
        if !state.entries.iter().any(|entry| entry == &self.record) {
            return Err(Error::Changed);
        }
        let expected = state.original.as_ref().ok_or(Error::Changed)?;
        let _guard = crate::session_file_guard::Guard::open(
            &self.store.0.path,
            expected,
            &AtomicBool::new(false),
        )
        .map_err(|_| Error::Changed)?;
        let mut active = self.store.0.activity.0.lock().map_err(|_| Error::Changed)?;
        if !active.insert(self.record.intent_id.clone()) {
            return Err(Error::Busy);
        }
        self.permit = Some(Permit {
            store: self.store.clone(),
            intent: self.record.intent_id.clone(),
        });
        Ok(())
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        self.store
            .0
            .activity
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .remove(&self.intent);
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "projection_receive_activity_tests.rs"]
mod tests;
