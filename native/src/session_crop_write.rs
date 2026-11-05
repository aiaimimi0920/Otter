//! 非破坏性裁剪元数据写回；源逻辑单位独立于像素和当前窗口尺寸，不负责图像变换。
use crate::crop_geometry::{self, Crop, Rect};
use serde_json::{json, Map, Value};

#[derive(Debug, PartialEq)]
pub enum Error {
    Shape,
    Geometry,
}

/// 仅拥有 cropRect/sourceSize；翻转、边框和擦除字段由对应图片内容 owner 处理。
/// 写出的是完整源框中的裁剪参数，不将其再次乘以窗口 DPI 或视口比例。
pub fn merge(original: Option<Value>, crop: Option<Crop>) -> Result<Value, Error> {
    let mut fields = match original {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(fields)) => fields,
        Some(_) => return Err(Error::Shape),
    };
    if let Some(crop) = crop {
        // 与正式读取器使用相同的几何合同；不钳制或取整小数源坐标。
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
        // 重置必须同时清理两项，不能让旧 sourceSize 在下一轮裁剪中复活。
        fields.remove("cropRect");
        fields.remove("sourceSize");
    }
    Ok(fields.into())
}

/// 活动Editor是唯一写回依据；legacy继续写自然像素区域，不伪造现代sourceSize。
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

#[cfg(test)]
#[path = "session_crop_write_tests.rs"]
mod tests;
