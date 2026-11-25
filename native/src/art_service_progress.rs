//! Host-only latest-progress slot: keep only status/percentage, never arbitrary remote text; not a final-result queue.
use super::{Reply, Status};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Update {
    pub status: Status,
    pub percent: Option<u8>,
}
impl Update {
    fn from_reply(reply: &Reply) -> Self {
        let percent = reply
            .progress
            .as_ref()
            .and_then(|value| value.get("percent"))
            .and_then(serde_json::Value::as_f64)
            .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
            .map(|value| value.floor() as u8);
        Self {
            status: reply.status,
            percent,
        }
    }
}
#[derive(Default)]
struct State {
    pending: Option<Update>,
    last: Option<Update>,
    closed: bool,
}
pub(crate) struct Channel {
    state: Mutex<State>,
    notify: Arc<dyn Fn() + Send + Sync>,
    pub confirmations: super::confirmation::Channel,
}
impl Channel {
    pub fn new(notify: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            confirmations: super::confirmation::Channel::new(notify.clone()),
            notify,
        }
    }
    pub fn publish(&self, reply: &Reply) {
        let value = Update::from_reply(reply);
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.closed || state.last == Some(value) {
            return;
        }
        let wake = state.pending.is_none();
        state.last = Some(value);
        state.pending = Some(value);
        drop(state);
        // Post only one Wake per unclaimed slot; continuous polling cannot accumulate unbounded UI messages.
        if wake {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (self.notify)()));
        }
    }
    pub fn take(&self) -> Option<Update> {
        self.state.lock().ok()?.pending.take()
    }
    pub fn clear(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.pending = None;
            state.last = None;
        }
    }
    pub fn close(&self) {
        self.confirmations.close();
        if let Ok(mut state) = self.state.lock() {
            state.closed = true;
            state.pending = None;
            state.last = None;
        }
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "art_service_progress_tests.rs"]
mod tests;
