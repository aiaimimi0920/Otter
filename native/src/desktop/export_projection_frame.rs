//! Map current crop back to validated original source coordinates, reusing only Frame::restored's natural output-density contract.
use super::*;
use crate::crop_geometry::{Crop, Rect, Size};

pub(super) fn prepare(
    image: &Image,
    editor: &Editor,
    logical: Size,
    source: Option<Size>,
    rasterized_source: bool,
) -> Result<source_resolution::Frame> {
    let Some(source) = source else {
        return Ok(
            source_resolution::Frame::from(logical).with_rasterized_source(rasterized_source)
        );
    };
    if ![source.width, source.height]
        .iter()
        .all(|value| value.is_finite() && *value > 0.0 && *value <= 1_000_000.0)
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Invalid projection source mapping",
        ));
    }
    let crop = editor
        .crop_view()
        // legacy is a natural-pixel stretch view, not modern sourceSize/contain cropping.
        .filter(|view| view.legacy_pixel_region().is_none())
        .map(|view| crate::session_crop::Restored {
            frame: Rect {
                x: 0.0,
                y: 0.0,
                ..view.source.rect
            },
            crop: Crop {
                source,
                rect: Rect {
                    x: view.source.rect.x * source.width / logical.width,
                    y: view.source.rect.y * source.height / logical.height,
                    width: view.source.rect.width * source.width / logical.width,
                    height: view.source.rect.height * source.height / logical.height,
                },
            },
            flipped_x: false,
            flipped_y: false,
            rasterized_source,
        });
    Ok(source_resolution::Frame::restored(image, logical, crop)?
        .with_rasterized_source(rasterized_source))
}
