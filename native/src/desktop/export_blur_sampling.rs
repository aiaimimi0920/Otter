//! Independent sampling contract for baseline blur annotations: ignore base flips/propagation frames and use source-pixel offsets directly for logical crops.
use super::*;

pub(super) fn original(image: &Image, flipped: (bool, bool), cancel: &AtomicBool) -> Result<Image> {
    // Call only on the first actual blur-resource request; without annotations, do not allocate a whole image just to undo base flips.
    let pixels = crate::raster_flip::image_axes(
        &image.pixels,
        (image.width, image.height),
        flipped.0,
        flipped.1,
        || cancel.load(Ordering::Acquire),
    )
    .map_err(|error| {
        Error::new(
            match error {
                crate::raster_flip::Error::Cancelled => windows::Win32::Foundation::E_ABORT,
                crate::raster_flip::Error::BudgetExceeded => {
                    windows::Win32::Foundation::E_OUTOFMEMORY
                }
                _ => E_INVALIDARG,
            },
            "Cannot restore blur source orientation",
        )
    })?;
    Ok(Image {
        width: image.width,
        height: image.height,
        pixels,
    })
}
