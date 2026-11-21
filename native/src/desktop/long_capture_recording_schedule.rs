//! 录制期间串行周期采样，覆盖拖动/滚动条/键盘移动；滚轮只加速，不作为位移证据。
use std::time::{Duration, Instant};
const DELAY: Duration = Duration::from_millis(24);
const POLL: Duration = Duration::from_millis(32);
const REPLY: Duration = Duration::from_millis(10);
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
        // 拖动没有 wheel 提示；立即完成也必须收取松手后的最后画面。
        if recording {
            self.final_sample = true;
            self.dirty = true;
            self.due.get_or_insert(now + DELAY);
        }
    }
    /// 只在唯一 worker 任务已被领取后调用；在途的新事件仍保留一个 dirty 位。
    pub fn completed(&mut self, now: Instant) {
        if !matches!(self.phase, Phase::Recording | Phase::Draining) {
            return;
        }
        if self.phase == Phase::Recording {
            // 保留采样开始时的截止；仅让出一个消息机会给预览，不再完成后额外等待一整周期。
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
        // 360ms 只约束额外尾采样；慢的在途任务不能挤掉尚未开始的完成后首张截图。
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
        // 待处理突发与周期截止分开；周期等待期间可以占用同一 worker 生成预览。
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
        // 帧数/像素上限归 SessionJob；周期抓到的重复帧不能耗尽保留帧额度。
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

#[cfg(test)]
#[path = "long_capture_recording_schedule_tests.rs"]
mod tests;
