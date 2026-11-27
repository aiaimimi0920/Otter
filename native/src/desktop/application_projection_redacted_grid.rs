//! Keep upstream redactions in their source coordinates without transferring
//! the upstream crop to the target or revealing what's outside that crop.
use crate::{crop_geometry::Rect, desktop::image::Image, editor::Editor, pixels, viewport};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG, E_OUTOFMEMORY},
};

pub(super) fn compose(image: &Image, editor: &Editor, cancel: &AtomicBool) -> Result<Image> {
    // The display renderer reads the authoritative working layers, including
    // erasure, blur and magnifier. Never read the retained unmarked original.
    let visible = crate::desktop::display_composition::compose(image, editor, cancel)?;
    let Some(view) = editor.crop_view() else {
        return Ok(visible);
    };
    let (w, h) = editor
        .layers()
        .map_or((image.width, image.height), |l| l.size);
    let bounds = editor.full_logical_frame().unwrap_or(Rect {
        x: 0.,
        y: 0.,
        width: f64::from(w),
        height: f64::from(h),
    });
    embed(&visible, view.source.rect, bounds, cancel)
}

fn embed(visible: &Image, crop: Rect, full: Rect, cancel: &AtomicBool) -> Result<Image> {
    let invalid = || Error::new(E_INVALIDARG, "Invalid redacted projection grid");
    if ![crop, full].into_iter().all(|r| {
        [r.x, r.y, r.width, r.height]
            .into_iter()
            .all(f64::is_finite)
            && r.width > 0.
            && r.height > 0.
    }) || viewport::checked_image_bytes(visible.width, visible.height)
        != Some(visible.pixels.len())
    {
        return Err(invalid());
    }
    let (width, height) = (
        full.width.round().max(1.) as u32,
        full.height.round().max(1.) as u32,
    );
    let size = viewport::checked_image_bytes(width, height).ok_or_else(invalid)?;
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(E_ABORT, "Redacted projection cancelled"));
    }
    let exhausted = || Error::new(E_OUTOFMEMORY, "Redacted projection grid exceeds budget");
    let mut lease = pixels::reserve(size).ok_or_else(exhausted)?;
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(size).map_err(|_| exhausted())?;
    if !lease.grow_to(bytes.capacity()) {
        return Err(exhausted());
    }
    bytes.resize(size, 0);
    // Map full-grid pixel centers to the safe crop. Outside stays transparent,
    // including fractional boundaries and legacy partially off-canvas crops.
    let pixel = |i: u32, start: f64, span: f64, count: u32, pos: f64, len: f64, n: u32| {
        let u = (start + (f64::from(i) + 0.5) * span / f64::from(count) - pos) / len;
        (0.0..1.0)
            .contains(&u)
            .then(|| ((u * f64::from(n)) as u32).min(n - 1) as usize)
    };
    for (y, row) in bytes.chunks_exact_mut(width as usize * 4).enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::new(E_ABORT, "Redacted projection cancelled"));
        }
        let Some(sy) = pixel(
            y as u32,
            full.y,
            full.height,
            height,
            crop.y,
            crop.height,
            visible.height,
        ) else {
            continue;
        };
        let start = sy * visible.width as usize * 4;
        for (x, dest) in row.chunks_exact_mut(4).enumerate() {
            if let Some(sx) = pixel(
                x as u32,
                full.x,
                full.width,
                width,
                crop.x,
                crop.width,
                visible.width,
            ) {
                let src = start + sx * 4;
                dest.copy_from_slice(&visible.pixels[src..src + 4]);
            }
        }
    }
    Ok(Image {
        width,
        height,
        pixels: pixels::Pixels::from_reserved(bytes, lease),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacted_crop_keeps_original_offset_and_never_fills_outside_with_natural_pixels() {
        let image = Image {
            width: 2,
            height: 1,
            pixels: vec![10, 20, 30, 255, 40, 50, 60, 255].into(),
        };
        let full = Rect {
            x: 0.,
            y: 0.,
            width: 5.,
            height: 3.,
        };
        for (offset, expected) in [(2.25, [0, 0, 10, 40, 0]), (-0.75, [40, 0, 0, 0, 0])] {
            let crop = Rect {
                x: offset,
                y: 1.,
                width: 2.,
                height: 1.,
            };
            let output = embed(&image, crop, full, &AtomicBool::new(false)).unwrap();
            assert_eq!((output.width, output.height), (5, 3));
            assert_eq!(&output.pixels[..20], &[0; 20]);
            assert_eq!(&output.pixels[40..], &[0; 20]);
            for (p, first) in output.pixels[20..40].chunks_exact(4).zip(expected) {
                assert_eq!((p[0], p[3]), (first, if first == 0 { 0 } else { 255 }));
            }
            assert_eq!(
                embed(&image, crop, full, &AtomicBool::new(true))
                    .err()
                    .unwrap()
                    .code(),
                E_ABORT
            );
        }
    }
}
