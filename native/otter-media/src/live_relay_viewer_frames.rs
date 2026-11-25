//! Viewer's three-frame window and terminal boundary; commit frame order and buffers under one lock so late frames cannot revive state after close.
use crate::{live_relay_frame::Metadata, live_relay_receive::Frame};
use std::{collections::VecDeque, sync::Mutex};
const CAPACITY: usize = 3;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Cursor {
    pub epoch: u64,
    pub frame_id: u64,
}
impl From<Metadata> for Cursor {
    fn from(value: Metadata) -> Self {
        Self {
            epoch: value.epoch,
            frame_id: value.frame_id,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidEpoch,
    Closed,
    Stale,
    Poisoned,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Commit {
    Accepted,
    IgnoredWarmup,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub cursor: Cursor,
    pub buffered: usize,
    pub overwritten: u64,
    pub closed: bool,
}
/// The cache only reads identity and grants no authorization; it neither copies nor changes the frame's backing storage.
pub trait CachedFrame {
    fn metadata(&self) -> Metadata;
}
impl CachedFrame for Frame {
    fn metadata(&self) -> Metadata {
        self.metadata()
    }
}
impl CachedFrame for crate::live_relay_pixels::PixelFrame {
    fn metadata(&self) -> Metadata {
        self.metadata()
    }
}
struct State<F> {
    cursor: Cursor,
    frames: VecDeque<F>,
    overwritten: u64,
    closed: bool,
}
pub struct FrameStore<F: CachedFrame>(Mutex<State<F>>);
pub type ViewerFrames = FrameStore<Frame>;
pub type ViewerPixels = FrameStore<crate::live_relay_pixels::PixelFrame>;
impl<F: CachedFrame> FrameStore<F> {
    /// Each authorized session holds its own owner; this module does not infer session authorization from media frames.
    pub fn new(epoch: u64) -> Result<Self, Error> {
        if epoch == 0 {
            return Err(Error::InvalidEpoch);
        }
        Ok(Self(Mutex::new(State {
            cursor: Cursor { epoch, frame_id: 0 },
            frames: VecDeque::with_capacity(CAPACITY),
            overwritten: 0,
            closed: false,
        })))
    }
    /// Compressed frames still require media validation; callers may commit only at permitted processing stages, not display directly on this basis.
    pub fn commit(&self, frame: F, allow_warmup: bool) -> Result<Commit, Error> {
        let mut state = self.0.lock().map_err(|_| Error::Poisoned)?;
        if state.closed {
            return Err(Error::Closed);
        }
        let cursor = Cursor::from(frame.metadata());
        if cursor <= state.cursor {
            return if allow_warmup {
                Ok(Commit::IgnoredWarmup)
            } else {
                Err(Error::Stale)
            };
        }
        if state.frames.len() == CAPACITY {
            state.frames.pop_front();
            state.overwritten = state.overwritten.saturating_add(1);
        }
        state.frames.push_back(frame);
        state.cursor = cursor;
        Ok(Commit::Accepted)
    }
    pub fn status(&self) -> Result<Status, Error> {
        let state = self.0.lock().map_err(|_| Error::Poisoned)?;
        Ok(Status {
            cursor: state.cursor,
            buffered: state.frames.len(),
            overwritten: state.overwritten,
            closed: state.closed,
        })
    }
    /// Compare both epoch and frame_id; a new epoch may restart frame numbering at 1.
    pub fn latest_after(&self, after: Cursor) -> Result<Option<Metadata>, Error> {
        let state = self.0.lock().map_err(|_| Error::Poisoned)?;
        if state.closed {
            return Err(Error::Closed);
        }
        Ok(state
            .frames
            .back()
            .map(CachedFrame::metadata)
            .filter(|metadata| Cursor::from(*metadata) > after))
    }
    /// Transfer the sole frame owner by complete identity and discard older cached frames; the lease lives with the output and is not returned early.
    pub fn take(&self, cursor: Cursor) -> Result<Option<F>, Error> {
        let mut state = self.0.lock().map_err(|_| Error::Poisoned)?;
        if state.closed {
            return Err(Error::Closed);
        }
        let Some(index) = state
            .frames
            .iter()
            .position(|frame| Cursor::from(frame.metadata()) == cursor)
        else {
            return Ok(None);
        };
        let frame = state.frames.drain(..=index).next_back();
        Ok(frame)
    }
    /// Do not wait for network threads. Even if another call panics and poisons the lock, clear local frames and terminate admission.
    pub fn close(&self) {
        let mut state = self.0.lock().unwrap_or_else(|poison| poison.into_inner());
        state.closed = true;
        state.frames.clear();
    }
}
#[cfg(test)]
#[path = "live_relay_viewer_pixels_tests.rs"]
mod pixel_tests;
#[cfg(test)]
#[path = "live_relay_viewer_frames_tests.rs"]
mod tests;
