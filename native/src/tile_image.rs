//! 验证 Wall 源摘要后复用已有受预算解码器；不落盘、不接收源 URL。
use crate::{image_decode::Image, tile_wire::Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use sha2::{Digest, Sha256};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    protocol_version: String,
    resource: Resource,
    data_base64: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Resource {
    resource_id: String,
    kind: String,
    mime: String,
    size: usize,
    width: Option<u32>,
    height: Option<u32>,
}
pub(crate) fn decode(bytes: &[u8], id: &str, stopped: impl Fn() -> bool) -> Result<Image> {
    const LIMIT: usize = 16 * 1024 * 1024;
    if bytes.len() > 24 * 1024 * 1024 {
        return Err("tile_image_limit");
    }
    let r: Response = serde_json::from_slice(bytes).map_err(|_| "tile_image_invalid")?;
    let d = r.resource;
    if r.protocol_version != crate::tile_wire::PROTOCOL
        || d.kind != "image"
        || !crate::tile_wire::image_id(id)
        || d.resource_id != id
        || d.size == 0
        || d.size > LIMIT
        || r.data_base64.len() > LIMIT.div_ceil(3) * 4
    {
        return Err("tile_image_invalid");
    }
    let data = STANDARD
        .decode(r.data_base64)
        .map_err(|_| "tile_image_invalid")?;
    if data.len() != d.size || format!("sha256:{:x}", Sha256::digest(&data)) != id {
        return Err("tile_image_integrity");
    }
    if ![
        "image/png",
        "image/jpeg",
        "image/webp",
        "image/bmp",
        "image/gif",
        "application/x-neuro-rgba8",
    ]
    .contains(&d.mime.as_str())
        || (d.mime != "application/x-neuro-rgba8"
            && crate::surface_image_format::encoded_mime(&data, &d.mime) != Some(d.mime.as_str()))
    {
        return Err("tile_image_format");
    }
    // Tile 必须在分配前限制单边与总像素，且声明尺寸不能悄悄与实际不符。
    let image = Image::from_tile(&data, &d.mime, d.width.zip(d.height), stopped)
        .map_err(|_| "tile_image_decode")?;
    if d.width.is_some_and(|w| w != image.width) || d.height.is_some_and(|h| h != image.height) {
        return Err("tile_image_dimensions");
    }
    Ok(image)
}
