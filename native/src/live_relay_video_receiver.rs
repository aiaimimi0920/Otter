//! 每媒体连接的同步视频 owner；调用线程须为 codec worker，不在 UI 回调执行。
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
    /// authorized_epoch 必须来自会话授权 owner，而非这个 frame。
    /// 此入口不授予权限；提交结果/发送控制前，调用方还须同锁复核授权及关闭状态。
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
                // 保留原 capture/encode/epoch 身份；输出表示是 raw，而不是压缩网络字节。
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
