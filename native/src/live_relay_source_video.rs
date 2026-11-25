//! Single-thread Source video selection; service-policy/reference-chain changes force IDR, and any codec failure makes raw sticky for this connection.
use crate::{
    byte_budget::Budget,
    live_mf::{Encoded, VideoEncoder},
    live_relay_media_transport::VideoPolicy,
};
use std::sync::{atomic::AtomicBool, Arc};
pub(super) struct Video {
    encoder: Option<VideoEncoder>,
    size: [u32; 2],
    policy: VideoPolicy,
    refresh: bool,
    since_idr: u32,
}
impl Video {
    pub fn new(size: [u32; 2], budget: &Arc<Budget>) -> Self {
        // Optional codecs cannot consume network quota required even by raw; the actual stream still holds its original lease.
        let network = budget.reserve(crate::live_relay_media_transport::BUFFER_BUDGET);
        Self {
            encoder: network.and_then(|_network| VideoEncoder::create(size, budget).ok()),
            size,
            policy: VideoPolicy {
                epoch: 1,
                h264_allowed: false,
                keyframe_sequence: 0,
            },
            refresh: false,
            since_idr: 30,
        }
    }
    pub fn available(&self) -> bool {
        self.encoder.is_some()
    }
    pub fn resize(&mut self, size: [u32; 2], budget: &Arc<Budget>) {
        if size == self.size {
            return;
        }
        self.size = size;
        self.since_idr = 30;
        // Release old encoder quota/system resources first; dimension changes cannot revive a failed encoder.
        if self.encoder.is_some() {
            self.encoder.take();
            self.encoder = VideoEncoder::create(size, budget).ok();
        }
    }
    pub fn legacy(&mut self) {
        self.encoder.take();
        self.policy.h264_allowed = false;
    }
    pub fn policy(&mut self, policy: VideoPolicy) -> Result<(), ()> {
        #[cfg(test)]
        eprintln!("source-video-policy: {policy:?}");
        if policy.epoch != 1 || policy.keyframe_sequence < self.policy.keyframe_sequence {
            return Err(());
        }
        if policy != self.policy {
            self.refresh = true;
            self.since_idr = 30;
        }
        self.policy = policy;
        Ok(())
    }
    pub fn refresh(&self) -> bool {
        self.refresh
    }
    pub fn reconnect(&mut self) {
        // Never replay lost AUs; send IDR after new policy, without reviving failed encoders on reconnect.
        self.policy.h264_allowed = false;
        self.refresh = true;
        self.since_idr = 30;
    }
    pub fn encode(&mut self, bgra: &[u8], frame_id: u64, stop: &AtomicBool) -> Option<Encoded> {
        self.refresh = false;
        if !self.policy.h264_allowed {
            self.since_idr = 30;
            return None;
        }
        let timestamp = i64::try_from(frame_id)
            .ok()
            .and_then(|n| n.checked_mul(10_000_000 / 30));
        let result = timestamp.and_then(|t| {
            self.encoder
                .as_mut()?
                .encode(bgra, t, self.since_idr >= 30, stop)
                .ok()
        });
        match result {
            Some(frame) => {
                self.since_idr = if frame.keyframe {
                    1
                } else {
                    self.since_idr + 1
                };
                Some(frame)
            }
            None => {
                self.encoder.take();
                None
            }
        }
    }
}
#[cfg(test)]
#[path = "live_relay_source_video_tests.rs"]
mod tests;
