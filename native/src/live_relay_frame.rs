//! 与 main NLLV v1 兼容的有界帧封装；不授权连接、不验证压缩码流、不分配像素。
pub const HEADER_LEN: usize = 64;
pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_JPEG_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Codec {
    RawBgra = 1,
    H264 = 2,
    Jpeg = 3,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ColorSpace {
    Srgb = 1,
    Hdr10 = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Metadata {
    pub epoch: u64,
    pub frame_id: u64,
    pub capture_timestamp_ms: u64,
    pub encode_timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    pub dropped_frames: u32,
    pub keyframe: bool,
    pub color_space: ColorSpace,
    pub codec: Codec,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Header,
    Identity,
    Timestamp,
    Dimensions,
    Payload,
    Profile,
}
impl Metadata {
    pub fn validate(self, payload_len: usize) -> Result<(), Error> {
        if self.epoch == 0 || self.frame_id == 0 {
            return Err(Error::Identity);
        }
        if self.encode_timestamp_ms < self.capture_timestamp_ms {
            return Err(Error::Timestamp);
        }
        if self.width == 0 || self.height == 0 || self.width > 16384 || self.height > 16384 {
            return Err(Error::Dimensions);
        }
        if payload_len == 0 || payload_len > MAX_FRAME_BYTES {
            return Err(Error::Payload);
        }
        let pixels = u64::from(self.width) * u64::from(self.height);
        let valid = match self.codec {
            Codec::RawBgra => pixels * 4 == payload_len as u64,
            Codec::Jpeg => {
                self.keyframe
                    && self.color_space == ColorSpace::Srgb
                    && payload_len <= MAX_JPEG_BYTES
                    && pixels * 4 <= MAX_FRAME_BYTES as u64
            }
            Codec::H264 => {
                self.color_space == ColorSpace::Srgb
                    && payload_len <= 1024 * 1024
                    && self.width >= 2
                    && self.height >= 2
                    && self.width <= 4096
                    && self.height <= 4096
                    && self.width.is_multiple_of(2)
                    && self.height.is_multiple_of(2)
                    && pixels <= 8_294_400
            }
        };
        if valid {
            Ok(())
        } else {
            Err(Error::Profile)
        }
    }
    /// 只生成固定头；发送者可与原 payload 分段写出，避免构造第二份整帧缓冲。
    pub fn encode_header(self, payload_len: usize) -> Result<[u8; HEADER_LEN], Error> {
        self.validate(payload_len)?;
        let mut header = [0; HEADER_LEN];
        header[..4].copy_from_slice(b"NLLV");
        header[4] = 1;
        header[5] = u8::from(self.keyframe);
        header[6..8].copy_from_slice(&(HEADER_LEN as u16).to_be_bytes());
        for (offset, value) in [
            (8, self.epoch),
            (16, self.frame_id),
            (24, self.capture_timestamp_ms),
            (32, self.encode_timestamp_ms),
        ] {
            header[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
        }
        for (offset, value) in [
            (40, self.width),
            (44, self.height),
            (48, self.dropped_frames),
            (52, payload_len as u32),
        ] {
            header[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
        }
        header[56] = self.color_space as u8;
        header[57] = self.codec as u8;
        Ok(header)
    }
}

/// 封装解析后的借用视图。压缩 payload 仍是不可信码流，必须在媒体层检查和解码。
pub struct Envelope<'a> {
    pub metadata: Metadata,
    pub payload: &'a [u8],
}
pub fn decode_envelope(bytes: &[u8]) -> Result<Envelope<'_>, Error> {
    let (metadata, length) = decode_header(bytes.get(..HEADER_LEN).ok_or(Error::Header)?)?;
    if length != bytes.len() - HEADER_LEN {
        return Err(Error::Payload);
    }
    Ok(Envelope {
        metadata,
        payload: &bytes[HEADER_LEN..],
    })
}
/// 可在读取/分配 payload 之前调用；长度上限及 codec 的尺寸约束在此生效。
pub fn decode_header(bytes: &[u8]) -> Result<(Metadata, usize), Error> {
    if bytes.len() != HEADER_LEN
        || &bytes[..4] != b"NLLV"
        || bytes[4] != 1
        || bytes[5] & !1 != 0
        || u16::from_be_bytes([bytes[6], bytes[7]]) as usize != HEADER_LEN
        || bytes[58..64].iter().any(|value| *value != 0)
    {
        return Err(Error::Header);
    }
    let read32 = |offset| {
        u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .expect("checked fixed header"),
        )
    };
    let read64 = |offset| {
        u64::from_be_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .expect("checked fixed header"),
        )
    };
    let length = read32(52) as usize;
    let metadata = Metadata {
        epoch: read64(8),
        frame_id: read64(16),
        capture_timestamp_ms: read64(24),
        encode_timestamp_ms: read64(32),
        width: read32(40),
        height: read32(44),
        dropped_frames: read32(48),
        keyframe: bytes[5] != 0,
        color_space: match bytes[56] {
            1 => ColorSpace::Srgb,
            2 => ColorSpace::Hdr10,
            _ => return Err(Error::Profile),
        },
        codec: match bytes[57] {
            1 => Codec::RawBgra,
            2 => Codec::H264,
            3 => Codec::Jpeg,
            _ => return Err(Error::Profile),
        },
    };
    metadata.validate(length)?;
    Ok((metadata, length))
}
#[cfg(test)]
#[path = "live_relay_frame_tests.rs"]
mod tests;
