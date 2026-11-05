//! 借用旧 sender 元数据做容量/重复准入；未识别或离线关联不被改写成原生在线资格。
use super::*;
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap};
#[derive(Deserialize)]
pub(super) struct Entries<'a>(#[serde(borrow, deserialize_with = "entries")] pub Vec<&'a RawValue>);
fn entries<'de: 'a, 'a, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Vec<&'a RawValue>, D::Error> {
    crate::session_document::bounded::<D, &'a RawValue, MAX_PER_UNIT>(decoder)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Legacy<'a> {
    #[serde(borrow)]
    link: LegacyLink<'a>,
    #[serde(borrow)]
    target: LegacyTarget<'a>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyLink<'a> {
    #[serde(borrow)]
    role: Cow<'a, str>,
    #[serde(borrow)]
    local_unit_id: Cow<'a, str>,
    #[serde(borrow)]
    envelope: &'a RawValue,
    revision: u64,
    #[serde(borrow)]
    digest: Cow<'a, str>,
    linked: bool,
    #[serde(borrow)]
    offline_transport: Option<Offline<'a>>,
}
#[derive(Deserialize)]
struct Offline<'a> {
    #[serde(borrow)]
    origin: Cow<'a, str>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyTarget<'a> {
    #[serde(borrow)]
    device_id: Cow<'a, str>,
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    peer_id: Option<Cow<'a, str>>,
}
pub(super) struct Key {
    pub envelope: Envelope,
    pub origin: String,
    pub projection: String,
    pub destination: (String, String, String),
}
pub(super) fn key(raw: &str, unit: &str) -> Result<Key, Error> {
    if raw.len() > MAX_BINDING_BYTES {
        return Err(Error::Limit);
    }
    let object = fields(raw).map_err(|_| Error::Format)?;
    fields(object.get("link").ok_or(Error::Format)?.get()).map_err(|_| Error::Format)?;
    fields(object.get("target").ok_or(Error::Format)?.get()).map_err(|_| Error::Format)?;
    let value: Legacy<'_> = serde_json::from_str(raw).map_err(|_| Error::Format)?;
    let link = value.link;
    let target = value.target;
    if link.role != "source"
        || link.local_unit_id != unit
        || link.envelope.get().len() > 8192
        || !envelope::identifier(&target.device_id)
        || target.name.len() > 1024
        || target.name.chars().any(char::is_control)
        || target.peer_id.is_some() != link.offline_transport.is_some()
        || target
            .peer_id
            .as_deref()
            .is_some_and(|id| !id.strip_prefix("loom-").is_some_and(envelope::digest))
    {
        return Err(Error::Format);
    }
    let envelope: Envelope =
        serde_json::from_str(link.envelope.get()).map_err(|_| Error::Format)?;
    if envelope.validate(envelope.origin()).is_err()
        || envelope.source_unit() != unit
        || !(envelope.source_revision()..=super::super::super::envelope::MAX_REVISION)
            .contains(&link.revision)
        || !envelope::digest(&link.digest)
        || (link.revision == envelope.source_revision() && link.digest != envelope.content_digest())
    {
        return Err(Error::Format);
    }
    let origin = link
        .offline_transport
        .as_ref()
        .map(|offline| offline.origin.as_ref())
        .unwrap_or(envelope.origin());
    if envelope::canonical_origin(origin).as_deref() != Ok(origin) {
        return Err(Error::Format);
    }
    let _ = link.linked; // awaiting_confirmation 的旧关联合法为 false，不以其推断远端未创建。
    Ok(Key {
        origin: envelope.origin().into(),
        projection: envelope.projection_id().into(),
        destination: (
            origin.into(),
            target.peer_id.as_deref().unwrap_or("").into(),
            target.device_id.to_string(),
        ),
        envelope,
    })
}
pub(super) fn native(raw: &str) -> Result<bool, Error> {
    if raw.len() > MAX_BINDING_BYTES {
        return Err(Error::Limit);
    }
    let object = fields(raw).map_err(|_| Error::Format)?;
    let link = fields(object.get("link").ok_or(Error::Format)?.get()).map_err(|_| Error::Format)?;
    Ok(link.contains_key("otterNativeSend"))
}
/// primary source 与数组合并计数；重复只允许同一个完整签名身份，不能借同 ID 掩盖冲突。
pub(super) fn primary(
    object: &BTreeMap<String, Cow<'_, RawValue>>,
    unit: &str,
    keys: &[Key],
) -> Result<(usize, Option<Envelope>), Error> {
    let Some(raw) = object.get("qrProjection").filter(|raw| raw.get() != "null") else {
        return Ok((0, None));
    };
    if raw.get().len() > MAX_BINDING_BYTES {
        return Err(Error::Limit);
    }
    let fields = fields(raw.get()).map_err(|_| Error::Format)?;
    let role: Cow<'_, str> = serde_json::from_str(fields.get("role").ok_or(Error::Format)?.get())
        .map_err(|_| Error::Format)?;
    if role == "receiver" {
        return Ok((0, None));
    }
    if role != "source" {
        return Err(Error::Format);
    }
    let link: LegacyLink<'_> = serde_json::from_str(raw.get()).map_err(|_| Error::Format)?;
    let envelope: Envelope =
        serde_json::from_str(link.envelope.get()).map_err(|_| Error::Format)?;
    if link.local_unit_id != unit
        || envelope.source_unit() != unit
        || envelope.validate(envelope.origin()).is_err()
        || !(envelope.source_revision()..=super::super::super::envelope::MAX_REVISION)
            .contains(&link.revision)
        || !envelope::digest(&link.digest)
        || (link.revision == envelope.source_revision() && link.digest != envelope.content_digest())
    {
        return Err(Error::Format);
    }
    let mut count = 1;
    if let Some(key) = keys
        .iter()
        .find(|key| key.origin == envelope.origin() && key.projection == envelope.projection_id())
    {
        if key.envelope != envelope {
            return Err(Error::Changed);
        }
        count = 0;
    }
    if keys.len() + count > MAX_PER_UNIT {
        return Err(Error::Limit);
    }
    Ok((count, Some(envelope)))
}
