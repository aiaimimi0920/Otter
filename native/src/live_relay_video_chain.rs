//! 每个媒体连接独占的 H264 参考链/描述符状态；不创建 codec、不发送控制消息。
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
    /// owner 在调用前验证会话未关闭/未撤销及 epoch；不能由媒体包隐式授予权限。
    /// Decode 只表示可以进入有界 SPS/AU 校验和 codec，并非已经解码成功。
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
    /// codec 输出必须和最早在途描述符匹配，不能把旧图片套到新的帧身份上。
    pub fn decoded(&mut self, timestamp: i64) -> Result<Metadata, Error> {
        let Some(metadata) = self.pending.pop_front() else {
            return self.reject(Error::UnexpectedOutput);
        };
        if i64::try_from(metadata.frame_id).ok() != Some(timestamp) {
            return self.reject(Error::UnexpectedOutput);
        }
        Ok(metadata)
    }
    /// codec/资源失败后返回是否首次降级。连接 owner 据此发送一次 video_fallback 并释放 codec。
    /// JPEG/raw 可继续到达，但不能取消此连接的 sticky fallback；重连须创建新 owner。
    pub fn fail(&mut self) -> bool {
        let first = !self.fallback_notified;
        self.reset();
        self.fallback = true;
        self.fallback_notified = true;
        first
    }
    fn reject<T>(&mut self, error: Error) -> Result<T, Error> {
        self.reset();
        // 返回错误交给 owner 调用 fail 并发送一次降级；不在本模块隐式发送控制消息。
        self.fallback = true;
        Err(error)
    }
}
#[cfg(test)]
#[path = "live_relay_video_chain_tests.rs"]
mod tests;
