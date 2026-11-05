//! Wall NLWM专属协议；直接编译复用旧生产校验器，不改造成普通NLLV帧。
use crate::image_decode::Image;
pub(crate) const PROTOCOL: &str = "loom.wall.media.v1";
pub(crate) const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WallMediaFormat {
    RawBgra,
    Png,
}
impl WallMediaFormat {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::RawBgra => "raw_bgra",
            Self::Png => "png",
        }
    }
}
#[path = "../../src-tauri/src/wall_live/packet.rs"]
mod wire;
pub(crate) const LIMIT: usize = MAX_FRAME_BYTES + wire::HEADER_LEN;
#[derive(Clone)]
pub(crate) struct Frame {
    pub identity: (u64, u64),
    pub received: u64,
    pub image: Image,
}
pub(crate) fn decode(
    bytes: &[u8],
    format: WallMediaFormat,
    previous: (u64, u64),
    received: u64,
    stopped: impl Fn() -> bool,
) -> Result<Frame, &'static str> {
    let (epoch, id, received) = wire::validate(bytes, format, previous, received)?;
    if stopped() {
        return Err("wall_live_stopped");
    }
    let width = u32::from_be_bytes(bytes[40..44].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[44..48].try_into().unwrap());
    let payload = &bytes[wire::HEADER_LEN..];
    let image = match format {
        WallMediaFormat::RawBgra => Image::from_bgra(payload, width, height),
        WallMediaFormat::Png => {
            Image::from_tile(payload, "image/png", Some((width, height)), &stopped)
        }
    }
    .map_err(|_| "wall_live_decode_failed")?;
    if stopped() || image.width != width || image.height != height {
        return Err("wall_live_dimensions_invalid");
    }
    Ok(Frame {
        identity: (epoch, id),
        received,
        image,
    })
}
#[cfg(test)]
#[path = "tile_live_packet_tests.rs"]
mod tests;
