//! Synchronous video owner per media connection; the calling thread must be a codec worker, not a UI callback.
use crate::{
    live_mf::{DecoderOwner, VideoDecoder},
    live_relay_frame::Codec,
    live_relay_pixels::PixelFrame,
    live_relay_receive::Frame,
    live_relay_video_chain::{Action, Chain},
};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::HRESULT;
pub enum Outcome {
    Image(Frame),
    Pending,
    RequestKeyframe { epoch: u64 },
    Fallback { epoch: u64, reason: Failure },
    Decoded(PixelFrame),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Chain,
    Codec(HRESULT),
    OutputIdentity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    Epoch,
}
#[derive(Default)]
pub struct Receiver {
    decoder: Option<VideoDecoder>,
    chain: Chain,
    closed: bool,
}
impl Receiver {
    pub fn close(&mut self) {
        self.decoder = None;
        self.chain = Chain::default();
        self.closed = true;
    }
    /// authorized_epoch must come from the session-authorization owner, not this frame.
    /// This entry point grants no permission; before committing results/sending control, callers must recheck authorization and closed state under the same lock.
    pub fn receive(
        &mut self,
        frame: Frame,
        authorized_epoch: u64,
        stop: &AtomicBool,
    ) -> Result<Outcome, Error> {
        if self.closed || stop.load(Ordering::Acquire) {
            self.close();
            return Err(Error::Closed);
        }
        let metadata = frame.metadata();
        if authorized_epoch == 0 || metadata.epoch != authorized_epoch {
            return Err(Error::Epoch);
        }
        let action = match self.chain.accept(&frame) {
            Ok(action) => action,
            Err(_) => return self.fail(metadata.epoch, Failure::Chain, stop),
        };
        match action {
            Action::Image => {
                self.decoder = None;
                Ok(Outcome::Image(frame))
            }
            Action::RequestKeyframe { epoch } => {
                self.decoder = None;
                Ok(Outcome::RequestKeyframe { epoch })
            }
            Action::WaitForKeyframe | Action::IgnoreFallback => {
                self.decoder = None;
                Ok(Outcome::Pending)
            }
            Action::Decode {
                reset_decoder,
                timestamp,
            } => {
                if reset_decoder {
                    self.decoder = None;
                }
                if self.decoder.is_none() {
                    match DecoderOwner::create()
                        .and_then(|owner| owner.configure(metadata.width, metadata.height))
                    {
                        Ok(owner) => self.decoder = Some(owner.into_video()),
                        Err(error) => {
                            return self.fail(metadata.epoch, Failure::Codec(error.code()), stop)
                        }
                    }
                }
                let result = self.decoder.as_mut().expect("initialized decoder").decode(
                    frame.payload(),
                    timestamp,
                    metadata.keyframe,
                    stop,
                );
                let output = match result {
                    Ok(Some(output)) => output,
                    Ok(None) => return Ok(Outcome::Pending),
                    Err(error) => {
                        return self.fail(metadata.epoch, Failure::Codec(error.code()), stop)
                    }
                };
                if stop.load(Ordering::Acquire) {
                    self.close();
                    return Err(Error::Closed);
                }
                let mut descriptor = match self.chain.decoded(output.timestamp) {
                    Ok(descriptor) => descriptor,
                    Err(_) => return self.fail(metadata.epoch, Failure::OutputIdentity, stop),
                };
                if (descriptor.width, descriptor.height) != (output.width, output.height) {
                    return self.fail(metadata.epoch, Failure::OutputIdentity, stop);
                }
                // Preserve the original capture/encode/epoch identity; the output representation is raw, not compressed network bytes.
                descriptor.codec = Codec::RawBgra;
                match PixelFrame::new(descriptor, output.pixels) {
                    Ok(frame) => Ok(Outcome::Decoded(frame)),
                    Err(_) => self.fail(metadata.epoch, Failure::OutputIdentity, stop),
                }
            }
        }
    }
    fn fail(&mut self, epoch: u64, reason: Failure, stop: &AtomicBool) -> Result<Outcome, Error> {
        self.decoder = None;
        if stop.load(Ordering::Acquire) {
            self.close();
            return Err(Error::Closed);
        }
        Ok(if self.chain.fail() {
            Outcome::Fallback { epoch, reason }
        } else {
            Outcome::Pending
        })
    }
}
#[cfg(test)]
#[path = "live_relay_video_receiver_tests.rs"]
mod tests;
