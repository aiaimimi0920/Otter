//! Official-image display pixels and optional original PNG share one immutable owner; aliases/snapshots clone only Arc.
//! PNGs must come from the same trusted encoding or decoded bytes; never publish arbitrary descriptors assembled with unrelated images.
use super::{export, image::Image};
use crate::pixels::Pixels;
use std::{
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};

#[derive(Clone)]
pub struct ImageValue {
    image: Image,
    png: Option<Pixels>,
}
impl From<Image> for ImageValue {
    fn from(image: Image) -> Self {
        Self { image, png: None }
    }
}
impl Deref for ImageValue {
    type Target = Image;
    fn deref(&self) -> &Image {
        &self.image
    }
}
impl ImageValue {
    pub fn image(&self) -> &Image {
        &self.image
    }
    pub fn encoded_png(&self) -> Option<&Pixels> {
        self.png.as_ref()
    }
    pub(crate) fn into_image(self) -> Image {
        self.image
    }
    pub(crate) fn bytes(&self) -> usize {
        self.image
            .pixels
            .len()
            .saturating_add(self.png.as_ref().map_or(0, |png| png.len()))
    }
    /// Callers prove pixels/PNGs share provenance; add geometry, format, and encoded-size admission here without decoding again.
    pub(crate) fn from_png(image: Image, png: Pixels) -> Result<Self> {
        if crate::viewport::checked_image_bytes(image.width, image.height)
            != Some(image.pixels.len())
            || !png.starts_with(b"\x89PNG\r\n\x1a\n")
            || png.len() > 64 * 1024 * 1024
        {
            return Err(Error::new(E_INVALIDARG, "Invalid formal PNG image"));
        }
        Ok(Self {
            image,
            png: Some(png),
        })
    }
    /// Background only. Preserve original PNGs; do not unpremultiply through PBGRA again and alter straight RGB.
    pub(crate) fn png(&self, cancel: &AtomicBool) -> Result<Pixels> {
        check(cancel)?;
        let png = match &self.png {
            Some(png) => png.clone(),
            None => export::png_raw(&self.image, cancel)?,
        };
        check(cancel)?;
        Ok(png)
    }
    /// Materialize inline data only; retain PNG bytes within shared budgets, without path, network, or user-profile access.
    pub(crate) fn inline(reference: &str, cancel: &AtomicBool) -> Result<Self> {
        let Some(encoded) = reference.strip_prefix("data:image/png;base64,") else {
            return super::session_image::inline(reference, cancel).map(Self::from);
        };
        Self::png_base64(encoded, cancel)
    }
    /// Workers reuse the same validated PNG without temporary data URLs or inferring original colors from PBGRA.
    pub(crate) fn png_base64(encoded: &str, cancel: &AtomicBool) -> Result<Self> {
        let png = super::loom_output_inline::decode_bytes("image/png", encoded, cancel)?;
        let _apartment = super::Apartment::new()?;
        let image = Image::from_encoded_with_cancel(&png, &|| cancel.load(Ordering::Acquire));
        check(cancel)?;
        Self::from_png(image?, png)
    }
}
fn check(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Formal PNG image cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_image_value_tests.rs"]
mod tests;
