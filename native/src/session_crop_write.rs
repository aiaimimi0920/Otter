//! Nondestructive crop-metadata writeback; source logical units are independent of pixels/current window size, without image transformation.
use crate::crop_geometry::{self, Crop, Rect};
use serde_json::{json, Map, Value};

#[derive(Debug, PartialEq)]
pub enum Error {
    Shape,
    Geometry,
}

/// Own only cropRect/sourceSize; corresponding image-content owners handle flip/border/erasure fields.
/// Write crop parameters within the complete source box, without multiplying by window DPI/viewport scale again.
pub fn merge(original: Option<Value>, crop: Option<Crop>) -> Result<Value, Error> {
    let mut fields = match original {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(fields)) => fields,
        Some(_) => return Err(Error::Shape),
    };
    if let Some(crop) = crop {
        // Use the same geometry contract as the production reader, without clamping/rounding fractional source coordinates.
        let frame = Rect {
            x: 0.0,
            y: 0.0,
            width: crop.rect.width,
            height: crop.rect.height,
        };
        crop_geometry::reset(frame, Some(crop)).map_err(|_| Error::Geometry)?;
        let mut rect = take_object(&mut fields, "cropRect")?;
        for (key, value) in [
            ("x", crop.rect.x),
            ("y", crop.rect.y),
            ("w", crop.rect.width),
            ("h", crop.rect.height),
        ] {
            rect.insert(key.into(), json!(value));
        }
        let mut source = take_object(&mut fields, "sourceSize")?;
        source.insert("w".into(), json!(crop.source.width));
        source.insert("h".into(), json!(crop.source.height));
        fields.insert("cropRect".into(), rect.into());
        fields.insert("sourceSize".into(), source.into());
    } else {
        // Reset must clear both fields so old sourceSize cannot revive in the next crop.
        fields.remove("cropRect");
        fields.remove("sourceSize");
    }
    Ok(fields.into())
}

/// Active Editor is the sole writeback source; legacy keeps natural-pixel regions without fabricating modern sourceSize.
pub fn merge_view(
    original: Option<Value>,
    view: Option<crate::editor::CropView>,
) -> Result<Value, Error> {
    let Some(pixels) = view.and_then(|view| view.legacy_pixel_region()) else {
        return merge(original, view.map(|view| view.source));
    };
    let mut fields = match original {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(fields)) => fields,
        Some(_) => return Err(Error::Shape),
    };
    let mut rect = take_object(&mut fields, "cropRect")?;
    for (key, value) in [
        ("x", pixels.x),
        ("y", pixels.y),
        ("w", pixels.width),
        ("h", pixels.height),
    ] {
        if !value.is_finite() || value.abs() > 1_000_000.0 {
            return Err(Error::Geometry);
        }
        rect.insert(key.into(), json!(value));
    }
    if pixels.width <= 0.0 || pixels.height <= 0.0 {
        return Err(Error::Geometry);
    }
    fields.insert("cropRect".into(), rect.into());
    fields.remove("sourceSize");
    Ok(fields.into())
}
fn take_object(fields: &mut Map<String, Value>, key: &str) -> Result<Map<String, Value>, Error> {
    match fields.remove(key) {
        None | Some(Value::Null) => Ok(Map::new()),
        Some(Value::Object(value)) => Ok(value),
        Some(_) => Err(Error::Shape),
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_crop_write_tests.rs"]
mod tests;
