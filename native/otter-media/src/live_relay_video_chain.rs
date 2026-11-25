//! H264 reference-chain/descriptor state owned exclusively by each media connection; creates no codec and sends no control messages.
use crate::{
    live_relay_frame::{Codec, Metadata},
    live_relay_receive::Frame,
};
use std::collections::VecDeque;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Image,
    Decode { reset_decoder: bool, timestamp: i64 },
    RequestKeyframe { epoch: u64 },
    WaitForKeyframe,
    IgnoreFallback,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Timeline,
    PendingLimit,
    UnexpectedOutput,
}
#[derive(Default)]
pub struct Chain {
    previous: Option<Metadata>,
    pending: VecDeque<Metadata>,
    fallback: bool,
    requested: Option<u64>,
    fallback_notified: bool,
}
impl Chain {
    fn reset(&mut self) {
        self.previous = None;
        self.pending.clear();
    }
    /// Before calling, the owner verifies that the session is not closed/revoked and checks the epoch; media packets cannot implicitly grant permission.
    /// Decode only permits entry into bounded SPS/AU validation and the codec; it does not mean decoding has succeeded.
    pub fn accept(&mut self, frame: &Frame) -> Result<Action, Error> {
        let metadata = frame.metadata();
        if metadata.codec != Codec::H264 {
            self.reset();
            self.requested = None;
            return Ok(Action::Image);
        }
        if self.fallback {
            return Ok(Action::IgnoreFallback);
        }
        let continuous = self.previous.is_some_and(|previous| {
            previous.epoch == metadata.epoch
                && previous.frame_id.checked_add(1) == Some(metadata.frame_id)
                && previous.width == metadata.width
                && previous.height == metadata.height
        });
        if !continuous {
            self.reset();
            if !metadata.keyframe {
                if self.requested != Some(metadata.epoch) {
                    self.requested = Some(metadata.epoch);
                    return Ok(Action::RequestKeyframe {
                        epoch: metadata.epoch,
                    });
                }
                return Ok(Action::WaitForKeyframe);
            }
        }
        if metadata.keyframe {
            self.requested = None;
        }
        let Ok(timestamp) = i64::try_from(metadata.frame_id) else {
            return self.reject(Error::Timeline);
        };
        if self.pending.len() >= 4 {
            return self.reject(Error::PendingLimit);
        }
        self.previous = Some(metadata);
        self.pending.push_back(metadata);
        Ok(Action::Decode {
            reset_decoder: !continuous,
            timestamp,
        })
    }
    /// Codec output must match the oldest in-flight descriptor; do not assign an old image a new frame identity.
    pub fn decoded(&mut self, timestamp: i64) -> Result<Metadata, Error> {
        let Some(metadata) = self.pending.pop_front() else {
            return self.reject(Error::UnexpectedOutput);
        };
        if i64::try_from(metadata.frame_id).ok() != Some(timestamp) {
            return self.reject(Error::UnexpectedOutput);
        }
        Ok(metadata)
    }
    /// After codec/resource failure, return whether this is the first fallback. The connection owner uses this to send video_fallback once and release the codec.
    /// JPEG/raw may keep arriving but cannot undo this connection's sticky fallback; reconnection requires a new owner.
    pub fn fail(&mut self) -> bool {
        let first = !self.fallback_notified;
        self.reset();
        self.fallback = true;
        self.fallback_notified = true;
        first
    }
    fn reject<T>(&mut self, error: Error) -> Result<T, Error> {
        self.reset();
        // Return the error for the owner to call fail and send fallback once; this module does not implicitly send control messages.
        self.fallback = true;
        Err(error)
    }
}
#[cfg(test)]
#[path = "live_relay_video_chain_tests.rs"]
mod tests;
