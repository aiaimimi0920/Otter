//! Relay static-frame decoding by background COM owners; H264 requires a separate stateful video decoder.
use super::image::Image;
use crate::{
    live_relay_frame::{Codec, ColorSpace},
    live_relay_receive::Frame,
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::E_INVALIDARG,
};
/// Static media and H264 share decoded-frame caches; transfer only existing WIC/raw pixel leases.
pub fn decode_pixels(frame: &Frame) -> Result<crate::live_relay_pixels::PixelFrame> {
    let image = decode(frame)?;
    let mut metadata = frame.metadata();
    metadata.codec = Codec::RawBgra;
    crate::live_relay_pixels::PixelFrame::new(metadata, image.pixels)
        .map_err(|_| Error::new(E_INVALIDARG, "Decoded relay image metadata mismatch"))
}
pub fn decode(frame: &Frame) -> Result<Image> {
    let metadata = frame.metadata();
    if metadata.color_space != ColorSpace::Srgb {
        return Err(Error::new(E_INVALIDARG, "Relay image requires sRGB"));
    }
    match metadata.codec {
        Codec::Jpeg => Image::from_relay_jpeg(frame.payload(), metadata.width, metadata.height),
        Codec::H264 => Err(Error::new(
            E_INVALIDARG,
            "H264 requires a stateful video decoder",
        )),
        Codec::RawBgra => Image::from_bgra(frame.payload(), metadata.width, metadata.height),
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "live_relay_image_tests.rs"]
mod tests;
