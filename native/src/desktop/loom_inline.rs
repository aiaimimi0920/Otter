//! Native committed-composite snapshots/official original PNGs to Loom inline input; no files, clipboard, or network.
use super::{export, image::Image, Apartment};
use crate::{editor::Editor, loom_art_wire::Input, loom_base64::Encoded, pixels::Pixels};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_FAIL},
};
pub struct Inline {
    encoded: Encoded,
    png: Pixels,
    width: u32,
    height: u32,
}
impl Inline {
    pub(crate) fn png(&self) -> Pixels {
        self.png.clone()
    }
    pub fn descriptor(&self) -> Input<'_> {
        Input::InlinePng {
            base64: self.encoded.as_str(),
            width: self.width,
            height: self.height,
        }
    }
}
/// Callers hold immutable editing snapshots in workers; WIC calls are uninterruptible, with cancellation checked at stage boundaries.
pub fn prepare(
    width: u32,
    height: u32,
    pixels: Pixels,
    editor: &Editor,
    cancel: &AtomicBool,
) -> Result<Inline> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(E_ABORT, "Loom input cancelled"));
    }
    let _apartment = Apartment::new()?;
    let image = Image {
        width,
        height,
        pixels,
    };
    let (png, width, height) = export::png_payload(&image, editor)?;
    from_png(width, height, png, cancel)
}
pub(crate) fn prepare_value(
    value: super::loom_image_value::ImageValue,
    cancel: &AtomicBool,
) -> Result<Inline> {
    let png = value.png(cancel)?;
    from_png(value.width, value.height, png, cancel)
}
fn from_png(width: u32, height: u32, png: Pixels, cancel: &AtomicBool) -> Result<Inline> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(E_ABORT, "Loom input cancelled"));
    }
    let encoded = crate::loom_base64::encode(&png, cancel).map_err(|error| {
        Error::new(
            if error == crate::loom_base64::Error::Cancelled {
                E_ABORT
            } else {
                E_FAIL
            },
            format!("Loom input encoding failed: {error:?}"),
        )
    })?;
    Ok(Inline {
        encoded,
        png,
        width,
        height,
    })
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_inline_tests.rs"]
mod tests;
