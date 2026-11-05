//! 滚轮是有界采样提示，不是位移证据。单 dirty 位合并输入，最多两次尾采样，无空闲扫描。
use std::time::{Duration, Instant};
const DELAY: Duration = Duration::from_millis(24);
const RECENT: Duration = Duration::from_millis(220);
const DRAIN: Duration = Duration::from_millis(360);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Recording,
    Stopped,
    Draining,
    Finished,
    Cancelled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Idle,
    Capture,
    Compose,
    Limit,
}
pub(super) struct Schedule {
    phase: Phase,
    started: Option<Instant>,
    latest: Option<Instant>,
    due: Option<Instant>,
    drain: Option<Instant>,
    dirty: bool,
    tail: u8,
    attempts: u32,
}
impl Default for Schedule {
    fn default() -> Self {
        Self {
            phase: Phase::Stopped,
            started: None,
            latest: None,
            due: None,
            drain: None,
            dirty: false,
            tail: 0,
            attempts: 0,
        }
    }
}
impl Schedule {
    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn start(&mut self, now: Instant) -> bool {
        if matches!(
            self.phase,
            Phase::Draining | Phase::Finished | Phase::Cancelled
        ) || self.limited(now)
        {
            return false;
        }
        self.started.get_or_insert(now);
        self.phase = Phase::Recording;
        true
    }
    pub fn notify(&mut self, now: Instant) {
        if self.phase != Phase::Recording {
            return;
        }
        self.latest = Some(now);
        self.dirty = true;
        self.tail = 2;
        self.due.get_or_insert(now + DELAY);
    }
    pub fn stop(&mut self) {
        if !matches!(self.phase, Phase::Finished | Phase::Cancelled) {
            self.phase = Phase::Stopped;
        }
        self.clear();
    }
    pub fn cancel(&mut self) {
        self.phase = Phase::Cancelled;
        self.clear();
    }
    pub fn finish(&mut self, now: Instant) {
        if matches!(self.phase, Phase::Finished | Phase::Cancelled) {
            return;
        }
        self.phase = Phase::Draining;
        self.drain = Some(now + DRAIN);
        if self
            .latest
            .is_none_or(|latest| now.saturating_duration_since(latest) > RECENT)
        {
            self.clear();
        }
    }
    /// 只在唯一 worker 任务已被领取后调用；在途的新事件仍保留一个 dirty 位。
    pub fn completed(&mut self, now: Instant) {
        if !matches!(self.phase, Phase::Recording | Phase::Draining) {
            return;
        }
        if !self.dirty
            && self.tail > 0
            && self
                .latest
                .is_some_and(|last| now.saturating_duration_since(last) <= RECENT)
        {
            self.tail -= 1;
            self.dirty = true;
            self.due = Some(now + DELAY);
        }
    }
    pub fn action(&mut self, now: Instant, busy: bool) -> Action {
        if busy || !matches!(self.phase, Phase::Recording | Phase::Draining) {
            return Action::Idle;
        }
        if self.limited(now) {
            if self.phase == Phase::Recording {
                self.stop();
                return Action::Limit;
            }
            self.clear();
        }
        if self.phase == Phase::Draining && self.drain.is_some_and(|end| now >= end) {
            self.clear();
        }
        if self.dirty && self.due.is_some_and(|due| now >= due) {
            if self.limited(now) {
                self.stop();
                return Action::Limit;
            }
            self.dirty = false;
            self.due = None;
            self.attempts += 1;
            return Action::Capture;
        }
        if self.phase == Phase::Draining && !self.dirty {
            self.phase = Phase::Finished;
            return Action::Compose;
        }
        Action::Idle
    }
    pub fn needs_timer(&self, busy: bool) -> bool {
        busy || self.dirty || self.phase == Phase::Draining
    }
    pub fn wake_after(&self, now: Instant, busy: bool) -> Option<Duration> {
        if busy {
            return Some(DELAY);
        }
        if let Some(due) = self.due {
            return Some(due.saturating_duration_since(now));
        }
        if self.phase == Phase::Draining {
            return Some(Duration::ZERO);
        }
        if self.phase == Phase::Recording {
            return self
                .started
                .map(|start| (start + Duration::from_secs(300)).saturating_duration_since(now));
        }
        None
    }
    fn limited(&self, now: Instant) -> bool {
        self.attempts >= 511
            || self.started.is_some_and(|start| {
                now.saturating_duration_since(start) >= Duration::from_secs(300)
            })
    }
    fn clear(&mut self) {
        self.dirty = false;
        self.tail = 0;
        self.due = None;
    }
}

#[cfg(test)]
#[path = "long_capture_recording_schedule_tests.rs"]
mod tests;
