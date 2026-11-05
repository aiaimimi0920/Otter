//! 工作流内容策略：匹配能力过滤参数，显式请求已编辑贴图烘焙，不回退泄漏原始内容。
use super::{check, Content, Error, Snapshot};
use crate::{loom_capability::Capability, session_window::Window};
use serde_json::{Map, Value};
use std::sync::atomic::AtomicBool;

/// bake 在后台准备原始分辨率合成；尺寸遵循主干缩略贴图 savedRect 规则。
pub fn prepare(
    saved: &[u8],
    revision: u64,
    catalog: &[Value],
    cancel: &AtomicBool,
    mut bake: impl FnMut(&Window<'_>, &Map<String, Value>, (f64, f64)) -> Result<String, Error>,
) -> Result<Snapshot, Error> {
    super::prepare(saved, revision, cancel, |window, fields| {
        check(cancel)?;
        let parameters = parameters(window, catalog)?;
        let image = if requires_bake(window, fields)? {
            let size = if window.is_minified() {
                window
                    .saved_rect
                    .map(|rect| (rect.w, rect.h))
                    .unwrap_or((window.w, window.h))
            } else {
                (window.w, window.h)
            };
            let src = bake(window, fields, size)?;
            check(cancel)?;
            if src.len() > crate::session_document::MAX_SESSION_BYTES
                || !src.starts_with("data:image/")
            {
                return Err(Error::Content);
            }
            Map::from_iter([
                ("src".into(), src.into()),
                ("rasterizedAnnotationLayerSrc".into(), Value::Null),
            ])
        } else {
            direct_image(window)
        };
        Ok(Content { parameters, image })
    })
}

pub(super) fn parameters(
    window: &Window<'_>,
    catalog: &[Value],
) -> Result<Map<String, Value>, Error> {
    if window.is_art() {
        let id = window.art_id.as_deref().ok_or(Error::Content)?;
        let mut matches = catalog
            .iter()
            .filter(|value| value["id"].as_str() == Some(id));
        let definition = matches.next().ok_or(Error::Content)?;
        if matches.next().is_some() {
            return Err(Error::Content);
        }
        let capability = Capability::new(definition).ok_or(Error::Content)?;
        let values = crate::session_parameters::restore(&capability, window.params)
            .map_err(|_| Error::Content)?;
        return Ok(values
            .entries()
            .filter_map(|(id, value)| value.map(|value| (id.into(), value.clone())))
            .collect());
    }
    crate::sticker_image_params::read(window.params).map_err(|_| Error::Content)
}

fn direct_image(window: &Window<'_>) -> Map<String, Value> {
    let mut image = Map::new();
    if let Some(src) = &window.src {
        image.insert("src".into(), src.as_ref().into());
    }
    let file = window
        .file_path
        .as_deref()
        .filter(|value| !value.is_empty());
    if let Some(file) = file {
        image.insert("filePath".into(), file.into());
    }
    if file.is_none() {
        if let Some(preview) = window
            .preview_src
            .as_deref()
            .filter(|value| !value.is_empty() && Some(*value) != window.src.as_deref())
        {
            image.insert("previewSrc".into(), preview.into());
        }
    }
    image.insert(
        "rasterizedAnnotationLayerSrc".into(),
        window
            .rasterized_annotation_layer_src
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(Value::from)
            .unwrap_or(Value::Null),
    );
    image
}

/// 对齐 requiresBakedStickerSyncImage；异常结构不当作无需烘焙而跳过。
fn requires_bake(window: &Window<'_>, fields: &Map<String, Value>) -> Result<bool, Error> {
    if window.is_art() {
        return Ok(false);
    }
    if let Some(annotation) = fields.get("annotationState").filter(|v| !v.is_null()) {
        let annotation = annotation.as_object().ok_or(Error::Content)?;
        if let Some(elements) = annotation.get("elements") {
            if !elements.as_array().ok_or(Error::Content)?.is_empty() {
                return Ok(true);
            }
        }
    }
    if window
        .rasterized_annotation_layer_src
        .as_deref()
        .is_some_and(|s| !s.is_empty())
        || window.opacity_normal.is_some_and(|value| value != 1.0)
    {
        return Ok(true);
    }
    let Some(edit) = fields.get("imageEditState").filter(|v| !v.is_null()) else {
        return Ok(false);
    };
    let edit = edit.as_object().ok_or(Error::Content)?;
    if let Some(strokes) = edit.get("contentEraseStrokes") {
        if !strokes.as_array().ok_or(Error::Content)?.is_empty() {
            return Ok(true);
        }
    }
    for key in [
        "cropRect",
        "sourceSize",
        "rotation",
        "flippedX",
        "flippedY",
        "borderWidth",
        "borderColor",
        "cornerRadius",
    ] {
        if edit.get(key).is_some_and(truthy) {
            return Ok(true);
        }
    }
    Ok(edit
        .get("beautify")
        .and_then(|v| v.get("enabled"))
        .is_some_and(truthy))
}
fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|n| n != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
#[path = "session_workflow_content_tests.rs"]
mod tests;
