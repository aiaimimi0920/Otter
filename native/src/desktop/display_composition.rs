//! Background display composition returns shared PBGRA pixels directly; reuse export drawing without PNG encode/decode round-trips.
use super::{export, image::Image, Apartment};
use crate::{editor::Editor, pixels::Pixels, viewport::checked_image_bytes};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::{
        Foundation::{E_ABORT, E_INVALIDARG, E_OUTOFMEMORY},
        Graphics::Imaging::{CLSID_WICImagingFactory, IWICImagingFactory},
        System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER},
    },
};

/// Callers retain immutable committed snapshots and invoke on workers; never return COM resources across threads.
/// WIC/Direct2D calls cannot be interrupted; check cancellation at composition/readback boundaries and deliver no partial pixels.
pub fn compose(image: &Image, editor: &Editor, cancel: &AtomicBool) -> Result<Image> {
    compose_reserved(image, editor, cancel, crate::pixels::reserve)
}

fn compose_reserved(
    image: &Image,
    editor: &Editor,
    cancel: &AtomicBool,
    reserve: impl Fn(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<Image> {
    check_cancel(cancel)?;
    if editor.draft().is_some() {
        return Err(Error::new(
            E_INVALIDARG,
            "Display composition requires committed snapshot",
        ));
    }
    let base = editor.layers().map(|layers| Image {
        width: layers.size.0,
        height: layers.size.1,
        pixels: layers.base.clone(),
    });
    let image = base.as_ref().unwrap_or(image);
    if checked_image_bytes(image.width, image.height) != Some(image.pixels.len()) {
        return Err(Error::new(
            E_INVALIDARG,
            "Invalid display composition image",
        ));
    }
    // Without visible edits, share the authoritative base image directly; do not rerender merely because undone operations remain in history.
    if editor.elements().is_empty()
        && editor.annotation_frame().is_none_or(|frame| {
            frame.x == 0.0
                && frame.y == 0.0
                && frame.width == image.width as f64
                && frame.height == image.height as f64
        })
        && editor
            .layers()
            .is_none_or(|layers| layers.annotations.is_none())
    {
        check_cancel(cancel)?;
        return Ok(image.clone());
    }
    let _apartment = Apartment::new()?;
    let wic: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)? };
    let output = export::compose_with_budget(image, editor, &wic, &reserve)?;
    check_cancel(cancel)?;
    let (mut width, mut height) = (0, 0);
    // Own composition bitmaps/output buffers locally; release COM and WIC budget leases after readback.
    unsafe {
        output.bitmap.GetSize(&mut width, &mut height)?;
    }
    let size = checked_image_bytes(width, height)
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid composed dimensions"))?;
    let mut lease = reserve(size)
        .ok_or_else(|| Error::new(E_OUTOFMEMORY, "Display readback budget exhausted"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|_| Error::new(E_OUTOFMEMORY, "Display readback allocation failed"))?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(Error::new(
            E_OUTOFMEMORY,
            "Display readback capacity exceeds budget",
        ));
    }
    bytes.resize(size, 0);
    unsafe {
        output
            .bitmap
            .CopyPixels(std::ptr::null(), width * 4, &mut bytes)?;
    }
    check_cancel(cancel)?;
    Ok(Image {
        width,
        height,
        pixels: Pixels::from_reserved(bytes, lease),
    })
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::new(E_ABORT, "Display composition cancelled"))
    } else {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "display_composition_tests.rs"]
mod tests;
