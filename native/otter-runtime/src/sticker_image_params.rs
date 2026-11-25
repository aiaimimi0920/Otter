//! 普通贴图独有的图片参数预算；不放宽 Art 参数或整个会话的容量/结构限制。
use serde_json::{Map, Value};

pub const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_URI_BYTES: usize = MAX_FILE_BYTES.div_ceil(3) * 4 + 64;
pub const MAX_FILENAME_BYTES: usize = 1024;

/// A checked inline payload borrowed from the exact string that passed admission.
pub struct InlineImage<'a> {
    mime: &'static str,
    encoded: &'a str,
}
impl<'a> InlineImage<'a> {
    pub fn parts(&self) -> (&'static str, &'a str) {
        (self.mime, self.encoded)
    }
}

pub fn image_parts(uri: &str) -> Result<(&'static str, &str), &'static str> {
    checked_image(uri).map(|image| image.parts())
}

pub fn checked_image(uri: &str) -> Result<InlineImage<'_>, &'static str> {
    if uri.len() > MAX_URI_BYTES {
        return Err("贴图图片超过32 MiB文件限制");
    }
    let (mime, encoded) = uri
        .strip_prefix("data:")
        .and_then(|value| value.split_once(";base64,"))
        .ok_or("贴图图片必须是内联Base64资源")?;
    let mime = match mime {
        "image/png" => "image/png",
        "image/jpeg" | "image/jpg" | "image/pjpeg" => "image/jpeg",
        "image/webp" => "image/webp",
        "image/bmp" | "image/x-ms-bmp" => "image/bmp",
        _ => return Err("贴图图片仅支持PNG、JPEG、WebP和BMP"),
    };
    if encoded.is_empty()
        || encoded.len() > MAX_FILE_BYTES.div_ceil(3) * 4
        || !encoded.len().is_multiple_of(4)
    {
        return Err("贴图图片Base64无效或超过容量");
    }
    let padding = encoded
        .bytes()
        .rev()
        .take_while(|byte| *byte == b'=')
        .count();
    if padding > 2
        || encoded[..encoded.len() - padding]
            .bytes()
            .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'+' && byte != b'/')
        || encoded.len() / 4 * 3 - padding > MAX_FILE_BYTES
    {
        return Err("贴图图片Base64无效或超过容量");
    }
    Ok(InlineImage { mime, encoded })
}

pub fn read(raw: Option<&serde_json::value::RawValue>) -> Result<Map<String, Value>, &'static str> {
    read_with_image(raw, |_, _| ()).map(|(values, ())| values)
}

/// Only expose the borrowed image after all parameter checks have passed.
pub fn read_with_image<T>(
    raw: Option<&serde_json::value::RawValue>,
    use_image: impl FnOnce(&Map<String, Value>, Option<InlineImage<'_>>) -> T,
) -> Result<(Map<String, Value>, T), &'static str> {
    let Some(raw) = raw else {
        let values = Map::new();
        let result = use_image(&values, None);
        return Ok((values, result));
    };
    if raw.get().len() > MAX_URI_BYTES + crate::loom_parameter_state::MAX_BYTES {
        return Err("贴图参数超过总容量");
    }
    let values: Map<String, Value> =
        serde_json::from_str(raw.get()).map_err(|_| "贴图参数必须为对象")?;
    let image = validate_with_image(&values)?;
    let result = use_image(&values, image);
    Ok((values, result))
}

pub fn validate(values: &Map<String, Value>) -> Result<(), &'static str> {
    validate_with_image(values).map(|_| ())
}

fn validate_with_image(
    values: &Map<String, Value>,
) -> Result<Option<InlineImage<'_>>, &'static str> {
    let image = if let Some(value) = values.get("image_path") {
        let uri = value.as_str().ok_or("贴图image_path必须为字符串")?;
        (!uri.is_empty()).then(|| checked_image(uri)).transpose()?
    } else {
        None
    };
    if values.get("image_filename").is_some_and(|value| {
        value.as_str().is_none_or(|name| {
            name.len() > MAX_FILENAME_BYTES || name.chars().any(char::is_control)
        })
    }) {
        return Err("贴图文件名无效或过长");
    }
    // 用空值替代唯一的大图字段，仍把该键计入128项上限；未知字段不获得大值例外。
    let empty = Value::Null;
    crate::loom_parameter_state::validate(values.iter().map(|(key, value)| {
        (
            key.as_str(),
            Some(if key == "image_path" { &empty } else { value }),
        )
    }))
    .map_err(|_| "贴图参数结构或容量无效")?;
    Ok(image)
}

#[cfg(test)]
#[path = "sticker_image_params_tests.rs"]
mod tests;
