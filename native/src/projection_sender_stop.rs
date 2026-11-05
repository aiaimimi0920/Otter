//! 稳定解绑的持久减权/成功回执仍归原 Store；回执不授网络能力，ack 只接受精确 canonical proof。
use super::*;
#[cfg(test)]
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
    /// 后台显式用户意图；两次 revoke 封住 activate 登记竞争，不能由普通退出隐式调用。
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
    /// proof 仅由 canonical commit owner 构造；这里仍后台重核完整日志基线和原成功回执。
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
