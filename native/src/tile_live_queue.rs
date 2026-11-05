//! 与Web同义的80ms软件延迟；最多三pending加一current，不声称硬件同步。
use crate::tile_live_packet::Frame;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
#[derive(Default)]
pub(crate) struct Queue {
    pending: VecDeque<Frame>,
    current: Option<Frame>,
    identity: (u64, u64),
    received: u64,
}
impl Queue {
    pub fn watermark(&self) -> ((u64, u64), u64) {
        (self.identity, self.received)
    }
    pub fn push(&mut self, frame: Frame) -> bool {
        if frame.identity <= self.identity || frame.received < self.received {
            return false;
        }
        if frame.identity.0 != self.identity.0 {
            self.clear();
        }
        self.identity = frame.identity;
        self.received = frame.received;
        self.pending.push_back(frame);
        if self.pending.len() > 3 {
            self.pending.remove(1);
        }
        true
    }
    fn clear(&mut self) {
        self.pending.clear();
        self.current = None;
    }
    pub fn advance(&mut self, estimate: Option<(u64, u64)>) -> Option<&Frame> {
        let Some((server, error)) = estimate else {
            self.clear();
            return None;
        };
        let target = server.saturating_sub(error).saturating_sub(80);
        let mut selected = None;
        while self.pending.front().is_some_and(|f| f.received <= target) {
            let frame = self.pending.pop_front().unwrap();
            if target - frame.received <= 250 {
                selected = Some(frame);
            }
        }
        if selected.is_some() {
            self.current = selected;
        } else if self
            .current
            .as_ref()
            .is_some_and(|f| target.saturating_sub(f.received) > 1000)
        {
            self.current = None;
        }
        self.current.as_ref()
    }
    /// UI独立watchdog：即使Assets被有界连接/解码阻塞，也不能无限显示旧Live。
    pub fn deadline(frame: &Frame, estimate: (u64, u64), now: Instant) -> Instant {
        let target = estimate.0.saturating_sub(estimate.1).saturating_sub(80);
        now + Duration::from_millis(frame.received.saturating_add(1000).saturating_sub(target))
    }
}
#[cfg(test)]
#[path = "tile_live_queue_tests.rs"]
mod tests;
