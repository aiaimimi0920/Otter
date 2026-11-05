//! 后台借用目标几何/传播字段；只物化当前步骤真正需要的小型编辑字段。
use super::*;
use serde::Deserialize;
use serde_json::{value::RawValue, Map, Value};
use std::borrow::Cow;
#[path = "export_projection_content.rs"]
mod content;
#[path = "export_projection_erase.rs"]
mod erase;

#[derive(Deserialize)]
struct Envelope<'a> {
    #[serde(borrow, deserialize_with = "units")]
    stickers: Vec<&'a RawValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Unit<'a> {
    #[serde(borrow)]
    id: Cow<'a, str>,
    x: Option<f64>,
    y: Option<f64>,
    #[serde(default, borrow)]
    image_edit_state: Option<&'a RawValue>,
    #[serde(default, borrow)]
    sticker_edit_propagation: Option<&'a RawValue>,
    w: Option<f64>,
    h: Option<f64>,
    minified: Option<bool>,
    saved_rect: Option<crate::session_window::Rect>,
}
pub(super) struct Target<'a> {
    id: Cow<'a, str>,
    pub edit: Value,
    pub logical: Option<crate::crop_geometry::Size>,
    pub hint: Option<crate::session_window::Rect>,
    edit_state: Option<&'a RawValue>,
    propagation: Option<&'a RawValue>,
}
fn units<'de: 'a, 'a, D: serde::Deserializer<'de>>(
    decoder: D,
) -> std::result::Result<Vec<&'a RawValue>, D::Error> {
    crate::session_document::bounded::<D, &'a RawValue, { crate::workspace::MAX_UNITS }>(decoder)
}
fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Invalid projection target appearance")
}
pub(super) fn read<'a>(
    source: &'a crate::session_source::Source,
    unit: &str,
) -> Result<Target<'a>> {
    read_selected(source, unit, true)
}
pub(super) fn read_baseline<'a>(
    source: &'a crate::session_source::Source,
    unit: &str,
) -> Result<Target<'a>> {
    read_selected(source, unit, false)
}
fn read_selected<'a>(
    source: &'a crate::session_source::Source,
    unit: &str,
    read_appearance: bool,
) -> Result<Target<'a>> {
    if source.len() > crate::session_document::MAX_SESSION_BYTES {
        return Err(invalid());
    }
    let envelope: Envelope<'_> = serde_json::from_slice(source).map_err(|_| invalid())?;
    let mut selected = None;
    for raw in envelope.stickers {
        let candidate: Unit<'_> = serde_json::from_str(raw.get()).map_err(|_| invalid())?;
        if candidate.id == unit {
            if selected.is_some() {
                return Err(invalid());
            }
            selected = Some(candidate);
        }
    }
    let selected = selected.ok_or_else(invalid)?;
    let hint = selected
        .x
        .zip(selected.y)
        .zip(selected.w.zip(selected.h))
        .map(|((x, y), (w, h))| crate::session_window::Rect { x, y, w, h });
    // 无本地 raster 的 Art/上游目标仍拥有自己的逻辑外框；不能按上游自然像素解释外观。
    let dimensions = if selected.minified == Some(true) {
        selected.saved_rect.map(|rect| (rect.w, rect.h))
    } else {
        None
    }
    .or(selected.w.zip(selected.h));
    let logical = dimensions.map(|(width, height)| crate::crop_geometry::Size { width, height });
    let Some(raw) = selected.image_edit_state else {
        return Ok(Target {
            id: selected.id,
            edit: Value::Null,
            logical,
            hint,
            edit_state: None,
            propagation: selected.sticker_edit_propagation,
        });
    };
    let fields = crate::session_json_fields::fields(raw.get()).map_err(|_| invalid())?;
    let mut appearance = Map::new();
    for key in ["borderWidth", "borderColor", "cornerRadius", "beautify"]
        .into_iter()
        .filter(|_| read_appearance)
    {
        if let Some(raw) = fields.get(key) {
            // 每个纯外观值先限字节，未知大历史/扩展不进入 Value，也不复制图片。
            if raw.get().len() > 16 * 1024 {
                return Err(invalid());
            }
            appearance.insert(
                key.into(),
                serde_json::from_str(raw.get()).map_err(|_| invalid())?,
            );
        }
    }
    Ok(Target {
        id: selected.id,
        edit: appearance.into(),
        logical,
        hint,
        edit_state: Some(raw),
        propagation: selected.sticker_edit_propagation,
    })
}

#[cfg(test)]
#[path = "export_projection_target_tests.rs"]
mod tests;
