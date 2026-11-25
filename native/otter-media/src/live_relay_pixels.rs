//! Decoded sRGB/PBGRA frames; transfer the pixel lease with the media identity without rewrapping the network payload.
use crate::{
    live_relay_frame::{Codec, ColorSpace, Error, Metadata},
    pixels::Pixels,
};
pub struct PixelFrame {
    metadata: Metadata,
    pixels: Pixels,
}
impl PixelFrame {
    /// Callers own actual decoding and the PBGRA convention; this checks that the descriptor matches the allocated pixel length.
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
