//! 显式PNG服务回复适配；独立有界网络envelope，原JSON合同不扩容，脚本不接触Base64。
use super::*;
use crate::surface_javascript::ImageResource;
use base64::engine::general_purpose::STANDARD;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub(super) const PROTOCOL: &str = "loom.art.local.service.image-output.v1";
const MAX_PNG: usize = 2 * 1024 * 1024;
pub(super) const MAX_REPLY: usize = wire::MAX_REPLY + 4 * MAX_PNG.div_ceil(3);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Envelope {
    protocol_version: String,
    reply: Reply,
    image: Option<Image>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Image {
    width: u32,
    height: u32,
    digest: String,
    png: String,
}
pub(super) fn encode(call: &Request<'_>) -> Result<Vec<u8>, Error> {
    serde_json::to_vec(&serde_json::json!({"protocolVersion":PROTOCOL,"call":call,"images":[]}))
        .map_err(|_| Error::Wire)
}
pub(super) fn parse(
    bytes: &[u8],
    owner: &Owner,
    prior: Option<&Reply>,
) -> Result<(Reply, Option<ImageResource>), Error> {
    if bytes.len() > MAX_REPLY {
        return Err(Error::Limit);
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|_| Error::Wire)?;
    if envelope.protocol_version != PROTOCOL {
        return Err(Error::Identity);
    }
    let reply = Reply::parse(
        &serde_json::to_vec(&envelope.reply).map_err(|_| Error::Wire)?,
        owner,
        prior,
    )?;
    if (reply.status == Status::Succeeded) != envelope.image.is_some() {
        return Err(Error::Wire);
    }
    let image = envelope
        .image
        .map(|image| decode(image, owner))
        .transpose()?;
    Ok((reply, image))
}
pub(super) fn decode(image: Image, owner: &Owner) -> Result<ImageResource, Error> {
    if image.png.len() > 4 * MAX_PNG.div_ceil(3) {
        return Err(Error::Limit);
    }
    let lease = crate::pixels::reserve(MAX_PNG + 3).ok_or(Error::Limit)?;
    let bytes = STANDARD.decode(image.png).map_err(|_| Error::Wire)?;
    if bytes.len() > MAX_PNG
        || bytes.len() < 24
        || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        || bytes[12..16] != *b"IHDR"
        || u32::from_be_bytes(bytes[16..20].try_into().unwrap()) != image.width
        || u32::from_be_bytes(bytes[20..24].try_into().unwrap()) != image.height
        || format!("{:x}", Sha256::digest(&bytes)) != image.digest
    {
        return Err(Error::Wire);
    }
    let resource = ImageResource {
        id: format!("service:{}", owner.call_id),
        width: image.width,
        height: image.height,
        png: crate::pixels::Pixels::from_reserved(bytes, lease),
    };
    crate::surface_javascript_wire::images::headers(std::slice::from_ref(&resource))
        .map_err(|_| Error::Limit)?;
    Ok(resource)
}
