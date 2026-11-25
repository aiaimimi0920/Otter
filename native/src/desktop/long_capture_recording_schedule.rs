//! Serial periodic sampling during recording covers dragging, scrollbars, and keyboard movement; wheel hints accelerate only, never prove displacement.
use std::time::{Duration, Instant};
const DELAY: Duration = Duration::from_millis(24);
const POLL: Duration = Duration::from_millis(32);
const REPLY: Duration = Duration::from_millis(10);
const RECENT: Duration = Duration::from_millis(220);
const DRAIN: Duration = Duration::from_millis(360);
const COVER_CHECK: Duration = Duration::from_millis(200);
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
    poll: Option<Instant>,
    drain: Option<Instant>,
    dirty: bool,
    final_sample: bool,
    tail: u8,
}
impl Default for Schedule {
    fn default() -> Self {
        Self {
            phase: Phase::Stopped,
            started: None,
            latest: None,
            due: None,
            poll: None,
            drain: None,
            dirty: false,
            final_sample: false,
            tail: 0,
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
        self.poll = Some(now + POLL);
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
    pub fn covered(&mut self, now: Instant) {
        // Do not repeatedly sample wheel bursts during occlusion; if the final finishing frame fails, deliver trusted frames directly.
        self.clear();
        self.latest = None;
        if self.phase == Phase::Recording {
            self.poll = Some(now + COVER_CHECK);
        }
    }
    pub fn cancel(&mut self) {
        self.phase = Phase::Cancelled;
        self.clear();
    }
    pub fn finish(&mut self, now: Instant) {
        if matches!(
            self.phase,
            Phase::Draining | Phase::Finished | Phase::Cancelled
        ) {
            return;
        }
        let recording = self.phase == Phase::Recording;
        self.phase = Phase::Draining;
        self.poll = None;
        self.drain = Some(now + DRAIN);
        if self
            .latest
            .is_none_or(|latest| now.saturating_duration_since(latest) > RECENT)
        {
            self.clear();
        }
        // Dragging has no wheel hint; immediate finish must still collect the final post-release image.
        if recording {
            self.final_sample = true;
            self.dirty = true;
            self.due.get_or_insert(now + DELAY);
        }
    }
    /// Call only after collecting the sole worker task; new in-flight events retain one dirty bit.
    pub fn completed(&mut self, now: Instant) {
        if !matches!(self.phase, Phase::Recording | Phase::Draining) {
            return;
        }
        if self.phase == Phase::Recording {
            // Keep the deadline from sampling start; yield one message opportunity to previews without waiting a full extra period after completion.
            self.poll = Some(self.poll.unwrap_or(now).max(now + Duration::from_millis(1)));
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
        // 360ms bounds additional tail sampling only; slow in-flight tasks must not displace the first not-yet-started post-finish capture.
        if self.phase == Phase::Draining
            && !self.final_sample
            && self.drain.is_some_and(|end| now >= end)
        {
            self.clear();
        }
        if (self.dirty && self.due.is_some_and(|due| now >= due))
            || (self.phase == Phase::Recording && self.poll.is_some_and(|due| now >= due))
        {
            if self.limited(now) {
                self.stop();
                return Action::Limit;
            }
            self.dirty = false;
            self.final_sample = false;
            self.due = None;
            self.poll = (self.phase == Phase::Recording).then_some(now + POLL);
            return Action::Capture;
        }
        if self.phase == Phase::Draining && !self.dirty {
            self.phase = Phase::Finished;
            return Action::Compose;
        }
        Action::Idle
    }
    pub fn needs_timer(&self, busy: bool) -> bool {
        // Separate pending bursts from periodic deadlines; previews may use the same worker during periodic waits.
        busy || self.dirty || self.phase == Phase::Draining
    }
    pub fn wake_after(&self, now: Instant, busy: bool) -> Option<Duration> {
        if busy {
            return Some(REPLY);
        }
        if let Some(due) = self.due.into_iter().chain(self.poll).min() {
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
        // SessionJob owns frame/pixel limits; duplicate periodic frames must not exhaust retained-frame quotas.
        self.started
            .is_some_and(|start| now.saturating_duration_since(start) >= Duration::from_secs(300))
    }
    fn clear(&mut self) {
        self.dirty = false;
        self.final_sample = false;
        self.tail = 0;
        self.due = None;
        self.poll = None;
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-capture")))]
#[path = "long_capture_recording_schedule_tests.rs"]
mod tests;
