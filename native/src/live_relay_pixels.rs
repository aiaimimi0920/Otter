//! 已解码的 sRGB/PBGRA 帧；像素租约随媒体身份一起移交，不重新封装网络 payload。
use crate::{
    live_relay_frame::{Codec, ColorSpace, Error, Metadata},
    pixels::Pixels,
};
pub struct PixelFrame {
    metadata: Metadata,
    pixels: Pixels,
}
impl PixelFrame {
    /// 调用方负责实际解码及 PBGRA 约定；这里验证描述符与已分配像素长度一致。
    pub fn new(metadata: Metadata, pixels: Pixels) -> Result<Self, Error> {
        if metadata.codec != Codec::RawBgra || metadata.color_space != ColorSpace::Srgb {
            return Err(Error::Profile);
        }
        metadata.validate(pixels.len())?;
        Ok(Self { metadata, pixels })
    }
    pub fn metadata(&self) -> Metadata {
        self.metadata
    }
    pub fn pixels(&self) -> &Pixels {
        &self.pixels
    }
    pub fn into_parts(self) -> (Metadata, Pixels) {
        (self.metadata, self.pixels)
    }
}
