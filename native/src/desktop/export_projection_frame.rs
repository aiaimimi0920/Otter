//! 当前crop映回已校验原源坐标，只复用原Frame::restored的自然输出密度合同。
use super::*;
use crate::crop_geometry::{Crop, Rect, Size};

pub(super) fn prepare(
    image: &Image,
    editor: &Editor,
    logical: Size,
    source: Option<Size>,
) -> Result<source_resolution::Frame> {
    let Some(source) = source else {
        return Ok(logical.into());
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
        // legacy是自然像素伸展视图，不是sourceSize/contain现代裁剪。
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
        });
    source_resolution::Frame::restored(image, logical, crop)
}
