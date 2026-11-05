//! 来源历史与目标布局独立；Region仅复用原采样器，成功后一次性交换几何和映射。
use super::*;

pub(in crate::desktop) fn prepare_mapped(
    image: &Image,
    geometry: &mut Frame,
    crop: Option<Rect>,
    edit: &serde_json::Value,
    history_mapping: &mut Sampling,
    cancel: &AtomicBool,
) -> Result<Image> {
    let Some(strokes) = strokes(edit, cancel)? else {
        return Ok(image.clone());
    };
    let layout = Layout::for_frame(image, geometry, crop)?;
    if matches!(geometry.sampling, Sampling::Empty) || matches!(history_mapping, Sampling::Empty) {
        return apply(
            image,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
            },
            true,
            strokes,
            cancel,
        );
    }
    let mapping = match *history_mapping {
        Sampling::Region(mapping) => mapping,
        Sampling::Full => Projection {
            source: bitmap_rect(image),
            target: bitmap_rect(image),
        },
        Sampling::Empty => unreachable!(),
    };
    validate(mapping, image)?;
    let natural = if let Sampling::Region(p) = layout.sampling {
        // 物化输出覆盖整个visible，不只是有效source区域；透明边距也参与同一仿射映射。
        Rect {
            x: p.source.x + (layout.visible.x - p.target.x) / p.target.width * p.source.width,
            y: p.source.y + (layout.visible.y - p.target.y) / p.target.height * p.source.height,
            width: layout.visible.width / p.target.width * p.source.width,
            height: layout.visible.height / p.target.height * p.source.height,
        }
    } else {
        bitmap_rect(image)
    };
    let frame = Rect {
        x: mapping.target.x
            + (natural.x - mapping.source.x) / mapping.source.width * mapping.target.width,
        y: mapping.target.y
            + (natural.y - mapping.source.y) / mapping.source.height * mapping.target.height,
        width: natural.width / mapping.source.width * mapping.target.width,
        height: natural.height / mapping.source.height * mapping.target.height,
    };
    valid_rect(frame)?;
    // 只复制采样几何小值；beautify由原owner持有，不Clone设置或共享像素。
    let mut pending = Frame {
        logical: geometry.logical,
        output: geometry.output,
        base: geometry.base,
        base_clip: geometry.base_clip,
        sampling: geometry.sampling,
        beautify: None,
        projection: geometry.projection,
    };
    let sampled = projection::prepare(image, &mut pending, crop, cancel)?;
    let source = sampled.as_ref().unwrap_or(image);
    let output = apply(source, frame, false, strokes, cancel)?;
    if cancel.load(Ordering::Acquire) {
        return Err(Error::new(
            windows::Win32::Foundation::E_ABORT,
            "Mapped source erase cancelled",
        ));
    }
    let mapping = Sampling::Region(Projection {
        source: bitmap_rect(&output),
        target: frame,
    });
    geometry.output = pending.output;
    geometry.base = pending.base;
    geometry.base_clip = pending.base_clip;
    geometry.sampling = pending.sampling;
    *history_mapping = mapping;
    Ok(output)
}

fn bitmap_rect(image: &Image) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        width: image.width as f64,
        height: image.height as f64,
    }
}
fn valid_rect(rect: Rect) -> Result<()> {
    if ![rect.x, rect.y, rect.width, rect.height]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.0)
        || rect.width <= 0.0
        || rect.height <= 0.0
    {
        return Err(Error::new(
            E_INVALIDARG,
            "History mapping exceeds geometry limits",
        ));
    }
    Ok(())
}
fn validate(mapping: Projection, image: &Image) -> Result<()> {
    valid_rect(mapping.source)?;
    valid_rect(mapping.target)?;
    if checked_image_bytes(image.width, image.height) != Some(image.pixels.len())
        || mapping.source.x < 0.0
        || mapping.source.y < 0.0
        || mapping.source.x + mapping.source.width > image.width as f64 + 0.000001
        || mapping.source.y + mapping.source.height > image.height as f64 + 0.000001
    {
        return Err(Error::new(E_INVALIDARG, "Invalid history source mapping"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "export_projection_mapped_erase_tests.rs"]
mod tests;
