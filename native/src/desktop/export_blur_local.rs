//! Sample/convolve each persistent effect frame independently; pixels outside frames are transparent, and output clipping is separate from white overlays.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Coordinates {
    Natural(Option<crate::session_crop::Restored>),
    WorkingGrid(Size),
}

pub(super) fn prepare(
    image: &Image,
    parent: &Layout,
    effect: Rect,
    coordinates: Coordinates,
    strength: f32,
    cancel: &AtomicBool,
) -> Result<BlurImage> {
    let sx = parent.output.0 as f64 / parent.visible.width;
    let sy = parent.output.1 as f64 / parent.visible.height;
    let output = (
        (effect.width.ceil() * sx).ceil() as u32,
        (effect.height.ceil() * sy).ceil() as u32,
    );
    if checked_image_bytes(output.0, output.1).is_none() {
        return Err(Error::new(E_INVALIDARG, "Local blur exceeds pixel limits"));
    }
    // Canvas first ceils logical dimensions, then pixel dimensions at output density; do not expand source projection to rounded bounds.
    let visible = Rect {
        x: 0.0,
        y: 0.0,
        width: output.0 as f64 / sx,
        height: output.1 as f64 / sy,
    };
    let sampling = projection(image, parent, effect, coordinates)?;
    let layout = Layout {
        output,
        visible,
        sampling,
        // Offscreen blur Canvas retains imageSmoothingEnabled and must not inherit the nearest-neighbor contract of base-image/erasure paths.
        interpolation: D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
        base: D2D_RECT_F::default(),
        base_clip: None,
        border: None,
        radius: 0.0,
    };
    let source = source_sample::readback(image, &layout, cancel)?;
    let strength = strength.round().max(1.0);
    let pixels = crate::blur::region_scaled(
        &source.pixels,
        output,
        crate::blur::Region {
            x: 0,
            y: 0,
            width: output.0,
            height: output.1,
        },
        strength,
        (sx as f32, sy as f32),
        || cancel.load(Ordering::Acquire),
    )
    .map_err(failure)?;
    let frame = Rect {
        x: parent.visible.x + effect.x,
        y: parent.visible.y + effect.y,
        ..effect
    };
    // CSS blur clips inside drawImage's target projection; later white overlays remain allowed outside that projection.
    let target = match sampling {
        Sampling::Region(projection) => projection.target,
        Sampling::Empty => Rect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        },
        Sampling::Full => unreachable!("local blur has explicit projection"),
    };
    let clip = Rect {
        x: frame.x + target.x / visible.width * frame.width,
        y: frame.y + target.y / visible.height * frame.height,
        width: target.width / visible.width * frame.width,
        height: target.height / visible.height * frame.height,
    };
    check(cancel)?;
    BlurImage::with_strength(output.0, output.1, pixels, strength)
        .and_then(|image| image.with_frame(frame))
        .and_then(|image| image.with_effect_clip(clip))
        .and_then(|image| {
            image.with_effect_mask_scale((
                (frame.width / visible.width) as f32,
                (frame.height / visible.height) as f32,
            ))
        })
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid local blur texture"))
}

fn projection(
    image: &Image,
    parent: &Layout,
    effect: Rect,
    coordinates: Coordinates,
) -> Result<Sampling> {
    let left = effect.x.max(0.0);
    let top = effect.y.max(0.0);
    let right = (effect.x + effect.width).min(parent.visible.width);
    let bottom = (effect.y + effect.height).min(parent.visible.height);
    if right <= left || bottom <= top {
        return Ok(Sampling::Empty);
    }
    let clipped = Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    };
    let target = Rect {
        x: left - effect.x,
        y: top - effect.y,
        ..clipped
    };
    let source = if let Coordinates::WorkingGrid(full) = coordinates {
        let sx = image.width as f64 / full.width;
        let sy = image.height as f64 / full.height;
        Rect {
            x: (parent.visible.x + left) * sx,
            y: (parent.visible.y + top) * sy,
            width: clipped.width * sx,
            height: clipped.height * sy,
        }
    } else if let Coordinates::Natural(Some(crop)) = coordinates {
        Rect {
            x: crop.crop.rect.x + left,
            y: crop.crop.rect.y + top,
            ..clipped
        }
    } else {
        let scale = (parent.visible.width / image.width as f64)
            .min(parent.visible.height / image.height as f64);
        let offset_x = (parent.visible.width - image.width as f64 * scale) / 2.0;
        let offset_y = (parent.visible.height - image.height as f64 * scale) / 2.0;
        Rect {
            x: (left - offset_x) / scale,
            y: (top - offset_y) / scale,
            width: clipped.width / scale,
            height: clipped.height / scale,
        }
    };
    Projection::clipped(source, target, (image.width, image.height))
        .ok_or_else(|| Error::new(E_INVALIDARG, "Invalid local blur projection"))
}
