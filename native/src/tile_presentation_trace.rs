//! 单个Tile生命期内每个固定阶段至多一条；原子位图无队列/字符串数据，不续期呈现授权。
use super::State;
use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Instant,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    Stopped,
    Identifying,
    Black,
    AuthorizationExpired,
    ClockUnavailable,
    ClockPending,
    LiveExpired,
    CandidateMissing,
    CandidateKeyMismatch,
    Visible,
    HiddenPaint,
    RendererMissing,
    PaintUnconfirmed,
    FramePaintUnconfirmed,
    PaintFailed,
    WindowRetiredDuringPaint,
    HiddenAfterPaint,
    KeyChangedDuringPaint,
    MediaExpiredDuringPaint,
    ArtChangedDuringPaint,
    Confirmed,
    HeartbeatPreparedWithApplied,
    HeartbeatPreparedWithoutApplied,
}

#[derive(Default)]
pub(crate) struct Trace(AtomicU32);
impl Trace {
    pub(crate) fn observe(&self, stage: Stage) {
        let bit = 1 << stage as u32;
        if self.0.load(Ordering::Relaxed) & bit != 0 {
            return;
        }
        // 只用于去重，无业务状态/权限的跨线程发布；释放Shared锁后才允许写stderr。
        if self.0.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
            eprintln!("native-tile-presentation: stage={stage:?}");
        }
    }
}

pub(super) fn visibility(s: &State, now: Instant) -> Stage {
    if s.stopped {
        Stage::Stopped
    } else if s.identification.blocked() {
        Stage::Identifying
    } else if s.black {
        Stage::Black
    } else if s.until.is_none_or(|v| now >= v) {
        Stage::AuthorizationExpired
    } else if !s.clock.permits(s.activate_at, now) {
        if s.clock.estimate(now).is_none() {
            Stage::ClockUnavailable
        } else {
            Stage::ClockPending
        }
    } else if !s.frozen
        && s.frame
            .as_ref()
            .is_none_or(|f| f.live_until.is_some_and(|v| now > v))
    {
        if s.frame.is_none() {
            Stage::CandidateMissing
        } else {
            Stage::LiveExpired
        }
    } else if s.frame.is_none() {
        Stage::CandidateMissing
    } else if !s
        .frame
        .as_ref()
        .is_some_and(|f| Some(&f.key) == s.desired.as_ref())
    {
        Stage::CandidateKeyMismatch
    } else {
        Stage::Visible
    }
}

#[cfg(test)]
#[path = "tile_presentation_trace_tests.rs"]
mod tests;
