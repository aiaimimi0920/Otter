//! Surface 输出描述符解析；资源 ID 不是下载授权，流 ID 不是订阅授权。
use serde::Deserialize;
use serde_json::value::RawValue;
use std::borrow::Cow;
pub const MAX_PORTS: usize = 64;
const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;
#[derive(Debug)]
pub struct Port<'a> {
    pub id: String,
    pub value: Value<'a>,
}
#[derive(Debug)]
pub enum Value<'a> {
    Value { value: &'a RawValue },
    Resource { resource: Resource<'a> },
    Stream { stream: Stream<'a> },
}
#[derive(Deserialize)]
struct Wire<'a> {
    #[serde(borrow)]
    kind: Cow<'a, str>,
    #[serde(default, borrow, deserialize_with = "present_raw")]
    value: Option<&'a RawValue>,
    #[serde(borrow)]
    resource: Option<Resource<'a>>,
    #[serde(borrow)]
    stream: Option<Stream<'a>>,
}
fn present_raw<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<&'de RawValue>, D::Error> {
    <&RawValue>::deserialize(decoder).map(Some)
}
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Image,
    Audio,
    Video,
    File,
    Binary,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource<'a> {
    #[serde(borrow)]
    pub resource_id: Cow<'a, str>,
    pub kind: ResourceKind,
    #[serde(borrow)]
    pub mime: Cow<'a, str>,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream<'a> {
    #[serde(borrow)]
    pub stream_id: Cow<'a, str>,
    #[serde(borrow)]
    pub item_type: Cow<'a, str>,
    #[serde(borrow)]
    pub mime: Option<Cow<'a, str>>,
    pub sequence: u64,
}
impl Value<'_> {
    fn valid(&self) -> bool {
        match self {
            Self::Value { .. } => true,
            Self::Resource { resource } => {
                let hash = resource.resource_id.strip_prefix("sha256:");
                hash.is_some_and(|hash| {
                    hash.len() == 64
                        && hash
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                }) && resource.mime.len() <= 128
                    && resource.size <= MAX_SAFE_INTEGER
                    && resource.width.is_none_or(|v| (1..=16384).contains(&v))
                    && resource.height.is_none_or(|v| (1..=16384).contains(&v))
            }
            Self::Stream { stream } => {
                crate::live_relay_viewer_identity::identifier(&stream.stream_id)
                    && stream.item_type.len() <= 160
                    && stream.mime.as_ref().is_none_or(|mime| mime.len() <= 128)
                    && stream.sequence <= MAX_SAFE_INTEGER
            }
        }
    }
}
/// 原文由 Commit 或调用方持有；value 大 JSON 只借用，最多分配 64 个端口描述符。
pub fn parse(raw: &RawValue) -> Result<Vec<Port<'_>>, crate::surface_result_commit::Error> {
    use crate::surface_result_commit::{Error, MAX_BYTES};
    if raw.get().len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    struct Ports;
    impl<'de> serde::de::Visitor<'de> for Ports {
        type Value = Vec<Port<'de>>;
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bounded typed Surface outputs")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut ports: Vec<Port<'de>> = Vec::new();
            while let Some(id) = map.next_key::<String>()? {
                if ports.len() == MAX_PORTS
                    || !crate::live_relay_viewer_identity::identifier(&id)
                    || ports.iter().any(|port| port.id == id)
                {
                    return Err(serde::de::Error::custom("Invalid Surface output keys"));
                }
                // 从借用原文单独解析，避免 internally-tagged enum 的中间 Value 复制 payload。
                let raw = map.next_value::<&RawValue>()?;
                let wire: Wire<'de> =
                    serde_json::from_str(raw.get()).map_err(serde::de::Error::custom)?;
                let value = match wire.kind.as_ref() {
                    "value" => wire.value.map(|value| Value::Value { value }),
                    "resource" => wire.resource.map(|resource| Value::Resource { resource }),
                    "stream" => wire.stream.map(|stream| Value::Stream { stream }),
                    _ => None,
                }
                .ok_or_else(|| serde::de::Error::custom("Missing Surface output value"))?;
                if !value.valid() {
                    return Err(serde::de::Error::custom(
                        "Invalid Surface output descriptor",
                    ));
                }
                ports.push(Port { id, value });
            }
            Ok(ports)
        }
    }
    let mut decoder = serde_json::Deserializer::from_str(raw.get());
    serde::Deserializer::deserialize_map(&mut decoder, Ports).map_err(|_| Error::Protocol)
}
#[cfg(test)]
#[path = "surface_result_ports_tests.rs"]
mod tests;
