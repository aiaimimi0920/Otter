//! 每次领取独立取消围栏；同一有界payload共享，不因派发/查询延长期限。
use super::*;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
pub(crate) struct Authorization(Mutex<bool>);
impl Authorization {
    pub fn new() -> Arc<Self> {
        Arc::new(Self(Mutex::new(true)))
    }
    pub fn stop(&self) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = false;
    }
    pub fn active(&self) -> bool {
        *self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}
struct State {
    binding: Binding,
    authorization: Arc<Authorization>,
    event: Event,
    deadline: Instant,
    observed_wall: AtomicU64,
    receipt: Mutex<Option<Receipt>>,
}
#[derive(Clone)]
pub(crate) struct Ticket(Arc<State>);
pub(crate) fn now() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis()
        .try_into()
        .ok()
}
impl Ticket {
    pub(super) fn new(
        binding: Binding,
        authorization: Arc<Authorization>,
        event: Event,
    ) -> Option<Self> {
        let now = now()?;
        if !event.validate()
            || now < event.delivery.observation.observed_at_ms
            || now >= event.delivery.expires_at_ms
        {
            return None;
        }
        let deadline = Instant::now() + Duration::from_millis(event.delivery.expires_at_ms - now);
        Some(Self(Arc::new(State {
            binding,
            authorization,
            event,
            deadline,
            observed_wall: AtomicU64::new(now),
            receipt: Mutex::new(None),
        })))
    }
    pub fn binding(&self) -> &Binding {
        &self.0.binding
    }
    pub fn event(&self) -> &Event {
        &self.0.event
    }
    fn time_current(&self) -> bool {
        let Some(now) = now() else { return false };
        self.time_current_at(now, Instant::now())
    }
    fn time_current_at(&self, now: u64, clock: Instant) -> bool {
        let previous = self.0.observed_wall.fetch_max(now, Ordering::AcqRel);
        now >= previous && now < self.0.event.delivery.expires_at_ms && clock < self.0.deadline
    }
    pub fn current(&self, binding: &Binding) -> bool {
        let valid =
            binding == &self.0.binding && self.0.authorization.active() && self.time_current();
        if !valid {
            self.0.authorization.stop();
        }
        valid
    }
    /// 只围住无Win32/网络/用户回调的canonical提交；停止与发布在同一锁串行化。
    pub fn commit<R>(
        &self,
        binding: &Binding,
        apply: impl FnOnce(&dyn Fn() -> bool) -> R,
    ) -> Option<R> {
        let mut active = self
            .0
            .authorization
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !*active || binding != &self.0.binding || !self.time_current() {
            *active = false;
            return None;
        }
        Some(apply(&|| self.time_current()))
    }
    /// 回执只在实际提交后写入；写一次，不能把迟到成功覆盖取消/失败。
    pub fn complete(&self, receipt: Receipt) -> bool {
        let mut active = self
            .0
            .authorization
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !*active || !self.time_current() {
            *active = false;
            return false;
        }
        let mut slot = self.0.receipt.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_some() {
            return false;
        }
        *slot = Some(receipt);
        true
    }
    pub(crate) fn receipt(&self) -> Option<Receipt> {
        self.0
            .receipt
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

#[cfg(test)]
#[path = "art_live_trigger_ticket_tests.rs"]
mod tests;
