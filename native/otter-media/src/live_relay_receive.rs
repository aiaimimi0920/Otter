//! Bounded single-frame receive for authorized media connections; does not create network connections or treat a complete envelope as a trusted bitstream.
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
    /// Must come from the actual WebSocket handshake selection, not payload guessing or automatic upgrades.
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

/// bytes is dropped before lease; downstream must still validate epoch/frame order and JPEG/H264 content before presentation.
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
    /// Take over the existing WebSocket allocation without copying the payload. The transport must limit message size before reading;
    /// this lease bounds only retained capacity after takeover and does not claim to cover WebSocket's own temporary allocation peak.
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
    /// Own only one frame at a time. The transport should read the fixed 64-byte header first, then read the body only after admission.
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
        // Budget actual Vec capacity; do not mistake logical length for all retained space that can be returned.
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
    /// Receive only this frame's payload; extra bytes cannot implicitly become the next frame, and overflow invalidates the whole candidate.
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
    /// Consume the sole owner; Drop reclaims truncation/cancellation without publishing partial content or adding a full-frame copy.
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
