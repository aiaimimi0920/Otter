//! 同意图的网络/Ready/消费所有权；不持 Mutex 跨 HTTP，Drop 只归还内存许可、不触盘。
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
    /// worker 发网前领取；许可贯穿 Ready→消费者，旧/重复 Ticket 不得并行解绑或重收。
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
#[cfg(test)]
#[path = "projection_receive_activity_tests.rs"]
mod tests;
