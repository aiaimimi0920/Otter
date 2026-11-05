//! 已授权媒体连接的有界单帧接收；不创建网络连接，也不把完整封装视为可信码流。
use crate::{
    byte_budget::{Budget, Lease},
    live_relay_frame::{self as wire, Codec, Metadata},
};
use std::sync::Arc;
pub const MEDIA_OFFER: &str = "loom.live.h264.v1,loom.live.jpeg.v1,loom.live.v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Legacy,
    Jpeg,
    H264,
}
impl Profile {
    /// 必须来自实际 WebSocket 握手的选择值，不从 payload 猜测或自动升级。
    pub fn negotiated(protocol: Option<&str>) -> Result<Self, Error> {
        match protocol {
            Some("loom.live.v1") => Ok(Self::Legacy),
            Some("loom.live.jpeg.v1") => Ok(Self::Jpeg),
            Some("loom.live.h264.v1") => Ok(Self::H264),
            _ => Err(Error::Profile),
        }
    }
    fn accepts(self, codec: Codec) -> bool {
        match codec {
            Codec::RawBgra => true,
            Codec::Jpeg => self != Self::Legacy,
            Codec::H264 => self == Self::H264,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Wire(wire::Error),
    Profile,
    Budget,
    Allocation,
    Length,
    Poisoned,
}

/// bytes 先于 lease 析构；下游仍需校验 epoch/帧序及 JPEG/H264 内容后才能呈现。
pub struct Frame {
    metadata: Metadata,
    bytes: Vec<u8>,
    offset: usize,
    _lease: Lease,
}
impl Frame {
    pub fn metadata(&self) -> Metadata {
        self.metadata
    }
    pub fn payload(&self) -> &[u8] {
        &self.bytes[self.offset..]
    }
    /// 接管 WebSocket 已有分配，不复制 payload。transport 必须在读取前限制消息大小；
    /// 此租约只约束接管后的保留容量，不声称覆盖 WebSocket 自身的临时分配峰值。
    pub fn from_message(
        bytes: Vec<u8>,
        profile: Profile,
        budget: &Arc<Budget>,
    ) -> Result<Self, Error> {
        let metadata = wire::decode_envelope(&bytes).map_err(Error::Wire)?.metadata;
        if !profile.accepts(metadata.codec) {
            return Err(Error::Profile);
        }
        let lease = budget.reserve(bytes.capacity()).ok_or(Error::Budget)?;
        Ok(Self {
            metadata,
            bytes,
            offset: wire::HEADER_LEN,
            _lease: lease,
        })
    }
}
pub struct PendingFrame {
    frame: Frame,
    received: usize,
    failed: bool,
}
impl PendingFrame {
    /// 一次只拥有一帧。transport 应先取固定 64 字节头，准入后才读取 body。
    pub fn begin(header: &[u8], profile: Profile, budget: &Arc<Budget>) -> Result<Self, Error> {
        let (metadata, length) = wire::decode_header(header).map_err(Error::Wire)?;
        if !profile.accepts(metadata.codec) {
            return Err(Error::Profile);
        }
        let mut lease = budget.reserve(length).ok_or(Error::Budget)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| Error::Allocation)?;
        // 预算核算 Vec 实际 capacity；不把逻辑长度误当可归还的全部保留空间。
        if !lease.grow_to(bytes.capacity()) {
            return Err(Error::Budget);
        }
        bytes.resize(length, 0);
        Ok(Self {
            frame: Frame {
                metadata,
                bytes,
                offset: 0,
                _lease: lease,
            },
            received: 0,
            failed: false,
        })
    }
    pub fn remaining(&self) -> usize {
        self.frame.bytes.len() - self.received
    }
    /// 只接收该帧 payload；多余字节不能隐式解释成下一帧，溢出后整个候选作废。
    pub fn push(&mut self, chunk: &[u8]) -> Result<(), Error> {
        if self.failed {
            return Err(Error::Poisoned);
        }
        if chunk.len() > self.remaining() {
            self.failed = true;
            return Err(Error::Length);
        }
        let end = self.received + chunk.len();
        self.frame.bytes[self.received..end].copy_from_slice(chunk);
        self.received = end;
        Ok(())
    }
    /// 消费唯一 owner；截断/取消由 Drop 回收，不发布部分内容，不增加整帧复制。
    pub fn finish(self) -> Result<Frame, Error> {
        if self.failed {
            return Err(Error::Poisoned);
        }
        if self.remaining() != 0 {
            return Err(Error::Length);
        }
        Ok(self.frame)
    }
}
#[cfg(test)]
#[path = "live_relay_receive_tests.rs"]
mod tests;
