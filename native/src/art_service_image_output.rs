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
    decode_with_budget(image, owner, crate::pixels::reserve)
}

fn decode_with_budget(
    image: Image,
    owner: &Owner,
    reserve: impl FnOnce(usize) -> Option<crate::byte_budget::Lease>,
) -> Result<ImageResource, Error> {
    if image.png.len() > 4 * MAX_PNG.div_ceil(3) {
        return Err(Error::Limit);
    }
    // 每四个 Base64 字符最多生成三字节；先为缓冲容量预约，再在原地解码。
    let capacity = image.png.len().div_ceil(4) * 3;
    let lease = reserve(capacity).ok_or(Error::Limit)?;
    let mut bytes = vec![0; capacity];
    let written = STANDARD
        .decode_slice(image.png, &mut bytes)
        .map_err(|_| Error::Wire)?;
    bytes.truncate(written);
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
mod tests {
    use super::*;
    use crate::byte_budget::Budget;
    use serde_json::json;

    #[test]
    fn image_output_budget_tracks_small_png_until_the_last_clone_drops() {
        let fixture = crate::surface_javascript_wire::images::tests::resource();
        let owner: Owner = serde_json::from_value(json!({
            "binding": {"origin":"https://localhost", "requestId":"request:1",
                "deviceId":"device:1", "workspaceId":"workspace:1", "instanceId":"instance:1",
                "generation":1, "artId":"vendor/art", "artVersion":"1.0.0",
                "packageDigest":"a".repeat(64), "sdkVersion":"otter.art.v1"},
            "grantId":"grant:1", "callId":"call:1", "service":"capability:vendor:service.run"
        }))
        .unwrap();
        let image = || Image {
            width: fixture.width,
            height: fixture.height,
            digest: format!("{:x}", Sha256::digest(&*fixture.png)),
            png: STANDARD.encode(&*fixture.png),
        };
        let len = fixture.png.len();
        let budget = Budget::new(len + 2);
        let reserve = |bytes| budget.reserve(bytes);
        let mut bad = image();
        bad.digest.clear();
        assert!(matches!(
            decode_with_budget(bad, &owner, reserve),
            Err(Error::Wire)
        ));
        assert_eq!(budget.used(), 0);

        let resource = decode_with_budget(image(), &owner, reserve).unwrap();
        assert_eq!(&*resource.png, &*fixture.png);
        let charged = budget.used();
        assert!((len..=len + 2).contains(&charged));
        assert!(matches!(
            decode_with_budget(image(), &owner, reserve),
            Err(Error::Limit)
        ));
        let clone = resource.png.clone();
        drop(resource);
        assert_eq!(budget.used(), charged);
        drop(clone);
        assert_eq!(budget.used(), 0);
    }
}
