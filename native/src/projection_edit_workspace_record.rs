//! 协作记录与同一canonical内容的有界摘要；流式读取图片字符串，不复制Base64。
use super::*;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Stored {
    snapshot: Snapshot,
    content_digest: String,
}
struct Hash(Sha256);
impl std::io::Write for Hash {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
const TEXT: [&str; 4] = [
    "src",
    "previewSrc",
    "filePath",
    "rasterizedAnnotationLayerSrc",
];
const JSON: [&str; 2] = ["annotationState", "imageEditState"];
fn digest<'a>(
    mut text: impl FnMut(&str) -> Option<&'a str>,
    mut json: impl FnMut(&str) -> Result<Option<Value>, Error>,
    size: (f64, f64),
) -> Result<String, Error> {
    let mut hash = Hash(Sha256::new());
    hash.0
        .update(b"neuro.otter.projection-edit-workspace-content.v1\0");
    for key in TEXT {
        hash.0.update(key.as_bytes());
        match text(key) {
            Some(value) => {
                hash.0.update([1]);
                hash.0.update((value.len() as u64).to_le_bytes());
                hash.0.update(value.as_bytes());
            }
            None => hash.0.update([0]),
        }
    }
    for key in JSON {
        hash.0.update(key.as_bytes());
        // 解析后的Value统一键序/空白；不按RawValue词法或f32近似比较。
        serde_json::to_writer(&mut hash, &json(key)?).map_err(|_| Error::Format)?;
    }
    hash.0.update(size.0.to_le_bytes());
    hash.0.update(size.1.to_le_bytes());
    Ok(format!("{:x}", hash.0.finalize()))
}
pub(super) fn install(snapshot: &Snapshot, fields: &mut Map<String, Value>) -> Result<(), Error> {
    let reference = if fields.get("minified").and_then(Value::as_bool) == Some(true) {
        fields
            .get("savedRect")
            .and_then(Value::as_object)
            .unwrap_or(&*fields)
    } else {
        &*fields
    };
    let size = (
        reference
            .get("w")
            .and_then(Value::as_f64)
            .ok_or(Error::Format)?,
        reference
            .get("h")
            .and_then(Value::as_f64)
            .ok_or(Error::Format)?,
    );
    let content_digest = digest(
        |key| fields.get(key).and_then(Value::as_str),
        |key| Ok(fields.get(key).cloned()),
        size,
    )?;
    let value = serde_json::to_value(Stored {
        snapshot: snapshot.clone(),
        content_digest,
    })
    .map_err(|_| Error::Format)?;
    if serde_json::to_vec(&value).map_or(true, |v| v.len() > MAX_RECORD) {
        return Err(Error::Limit);
    }
    fields.insert(FIELD.into(), value);
    Ok(())
}
pub(super) fn read(
    raw: &RawValue,
    window: &crate::session_window::Window<'_>,
) -> Result<Snapshot, Error> {
    if raw.get().len() > MAX_RECORD {
        return Err(Error::Limit);
    }
    let stored: Stored = serde_json::from_str(raw.get()).map_err(|_| Error::Format)?;
    let frame = window.expanded_rect().ok_or(Error::Format)?;
    let content = digest(
        |key| match key {
            "src" => window.src.as_deref(),
            "previewSrc" => window.preview_src.as_deref(),
            "filePath" => window.file_path.as_deref(),
            "rasterizedAnnotationLayerSrc" => window.rasterized_annotation_layer_src.as_deref(),
            _ => None,
        },
        |key| {
            let raw = match key {
                "annotationState" => window.annotation_state,
                _ => window.image_edit_state,
            };
            raw.map(|raw| serde_json::from_str(raw.get()).map_err(|_| Error::Format))
                .transpose()
        },
        (frame.w, frame.h),
    )?;
    if !super::super::hex(&stored.content_digest, 64) || content != stored.content_digest {
        return Err(Error::ContentChanged);
    }
    Ok(stored.snapshot)
}
