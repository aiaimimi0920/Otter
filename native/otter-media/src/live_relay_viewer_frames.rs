//! Viewer 的三帧窗口与终止边界；同一锁提交帧序和缓冲，避免 close 后迟到帧复活。
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
/// 缓存只读取身份，不赋予授权；不复制或改变帧的底层存储。
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
    /// 每个授权会话单独持有一个 owner；本模块不从媒体帧推导会话授权。
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
    /// 压缩帧仍须媒体验证；调用方只能在允许的处理阶段提交，不能据此直接显示。
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
    /// 同时比较 epoch 和 frame_id；新 epoch 的帧号可以重新从 1 开始。
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
    /// 按完整身份移交唯一 frame owner，丢弃更早缓存；租约随输出存活，不提前归还。
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
    /// 不等待网络线程。即使其他调用 panic 导致锁 poison，也必须清空本地帧并终止准入。
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
