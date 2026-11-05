//! Loom 服务回复的独立客户端 DTO；身份来自宿主请求，绝不从回复补造授权。
use crate::art_execution_grant::wire::Binding;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub(super) const PROTOCOL: &str = "loom.art.local.service.v1";
pub(super) const MAX_INPUT: usize = 64 * 1024;
pub(super) const MAX_REPLY: usize = 96 * 1024;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Owner {
    pub binding: Binding,
    pub grant_id: String,
    pub call_id: String,
    pub service: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}
impl Status {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reply {
    pub protocol_version: String,
    pub owner: Owner,
    pub sequence: u64,
    pub status: Status,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "output"
    )]
    pub output: Option<Value>,
    pub error: Option<String>,
    pub progress: Option<Value>,
}
fn output<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}
fn bounded(value: &Option<Value>, limit: usize) -> bool {
    value
        .as_ref()
        .is_none_or(|v| serde_json::to_vec(v).is_ok_and(|bytes| bytes.len() <= limit))
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
impl Reply {
    pub(super) fn parse(
        bytes: &[u8],
        owner: &Owner,
        prior: Option<&Self>,
    ) -> Result<Self, super::Error> {
        use super::Error;
        if bytes.len() > MAX_REPLY {
            return Err(Error::Limit);
        }
        let reply: Self = serde_json::from_slice(bytes).map_err(|_| Error::Wire)?;
        if reply.protocol_version != PROTOCOL || &reply.owner != owner {
            return Err(Error::Identity);
        }
        if reply.sequence == 0 || reply.sequence > 9_007_199_254_740_991 {
            return Err(Error::Sequence);
        }
        let valid = match reply.status {
            Status::Succeeded => reply.output.is_some() && reply.error.is_none(),
            Status::Failed => {
                reply.output.is_none() && reply.error.as_deref().is_some_and(identifier)
            }
            _ => reply.output.is_none() && reply.error.is_none(),
        };
        if !valid {
            return Err(Error::Wire);
        }
        if !bounded(&reply.output, MAX_INPUT) || !bounded(&reply.progress, 4096) {
            return Err(Error::Limit);
        }
        if let Some(prior) = prior {
            if reply.sequence < prior.sequence
                || (reply.sequence == prior.sequence && &reply != prior)
                || (prior.status.terminal() && &reply != prior)
                || (prior.status == Status::Running && reply.status == Status::Accepted)
            {
                return Err(Error::Sequence);
            }
        }
        Ok(reply)
    }
}

#[cfg(test)]
#[path = "art_service_wire_tests.rs"]
mod tests;
