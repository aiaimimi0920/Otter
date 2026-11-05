//! 经过包证明和摘要核验的资源接入原Content；命名空间隔离，图片复用原二进制安装与预算。
use crate::{
    art_execution_grant::package::Resource,
    pixels::Pixels,
    surface_javascript::{self as js, signed::runtime::Content},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
pub(crate) struct Asset {
    pub resource: Resource,
    pub bytes: Pixels,
}
impl Asset {
    pub(crate) fn accept(resource: Resource, bytes: Vec<u8>) -> Result<Self, ()> {
        crate::art_execution_grant::package::validate_resources(std::slice::from_ref(&resource))
            .map_err(|_| ())?;
        if bytes.len() as u64 != resource.size
            || format!("{:x}", Sha256::digest(&bytes)) != resource.digest
        {
            return Err(());
        }
        if matches!(
            resource.mime_type.as_str(),
            "text/plain" | "application/json"
        ) {
            if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
                return Err(());
            }
            if resource.mime_type == "application/json"
                && serde_json::from_slice::<Value>(&bytes).is_err()
            {
                return Err(());
            }
        }
        if resource.mime_type == "image/png"
            && (bytes.len() < 33
                || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                || &bytes[12..16] != b"IHDR"
                || u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| ())?)
                    != resource.dimensions[0]
                || u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| ())?)
                    != resource.dimensions[1])
        {
            return Err(());
        }
        let lease = crate::pixels::reserve(bytes.capacity()).ok_or(())?;
        Ok(Self {
            resource,
            bytes: Pixels::from_reserved(bytes, lease),
        })
    }
}
pub(crate) fn merge(mut content: Content, assets: &[Asset]) -> Result<Content, ()> {
    if assets.is_empty() {
        return Ok(content);
    }
    let resources = content.resources.as_object_mut().ok_or(())?;
    let state = content
        .snapshot
        .get_mut("state")
        .and_then(Value::as_object_mut)
        .ok_or(())?;
    if state.contains_key("package") {
        return Err(());
    }
    let mut package = Map::new();
    for asset in assets {
        let r = &asset.resource;
        let id = format!("art-package:{}", r.id);
        if resources.contains_key(&id) || content.images.iter().any(|i| i.id == id) {
            return Err(());
        }
        if r.mime_type == "image/png" {
            content.images.push(js::ImageResource {
                id: id.clone(),
                width: r.dimensions[0],
                height: r.dimensions[1],
                png: asset.bytes.clone(),
            });
        } else {
            let value = if r.mime_type == "application/octet-stream" {
                format!(
                    "data:application/octet-stream;base64,{}",
                    STANDARD.encode(&*asset.bytes)
                )
            } else {
                std::str::from_utf8(&asset.bytes)
                    .map_err(|_| ())?
                    .to_owned()
            };
            resources.insert(id.clone(), Value::String(value));
        }
        package.insert(r.id.clone(), json!({"resourceId":id,"mimeType":r.mime_type,"size":r.size,"digest":r.digest,"width":r.dimensions[0],"height":r.dimensions[1]}));
    }
    state.insert("package".into(), Value::Object(package));
    js::resources(&content.resources).map_err(|_| ())?;
    if !content.images.is_empty() {
        crate::surface_javascript_wire::images::headers(&content.images).map_err(|_| ())?;
    }
    Ok(content)
}
#[cfg(test)]
#[path = "art_package_content_tests.rs"]
mod tests;
