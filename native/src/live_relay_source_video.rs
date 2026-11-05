//! Source单线程视频选择；服务策略/参考链变化强制IDR，任何codec失败在本连接sticky raw。
use crate::{
    byte_budget::Budget,
    live_mf::{Encoded, VideoEncoder},
    live_relay_media_transport::VideoPolicy,
};
use std::sync::{atomic::AtomicBool, Arc};
pub(super) struct Video {
    encoder: Option<VideoEncoder>,
    policy: VideoPolicy,
    refresh: bool,
    since_idr: u32,
}
impl Video {
    pub fn new(size: [u32; 2], budget: &Arc<Budget>) -> Self {
        // 可选codec不能挤掉raw也必需的网络额度；实际stream仍自己持有原租约。
        let network = budget.reserve(crate::live_relay_media_transport::BUFFER_BUDGET);
        Self {
            encoder: network.and_then(|_network| VideoEncoder::create(size, budget).ok()),
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
        // 丢失的AU不能重放；等新策略后发IDR，失败的encoder不因重连复活。
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
