//! 只交付匹配请求的完整成功结果；副作用是待审核的数据，不在网络层执行系统或文档操作。
use super::{catalog, Error};
use serde::Deserialize;
use serde_json::Value;
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum EffectKind {
    #[serde(rename = "attachment.upsert")]
    AttachmentUpsert,
    #[serde(rename = "attachment.remove")]
    AttachmentRemove,
    #[serde(rename = "notice.show")]
    NoticeShow,
    #[serde(rename = "clipboard.writeText")]
    ClipboardWriteText,
    #[serde(rename = "external.openUrl")]
    ExternalOpenUrl,
    #[serde(rename = "overlay.invalidate")]
    OverlayInvalidate,
    #[serde(rename = "resource.publish")]
    ResourcePublish,
}
#[derive(Debug, Deserialize)]
pub struct Effect {
    #[serde(rename = "type")]
    kind: EffectKind,
    #[serde(default)]
    payload: Value,
}
impl Effect {
    pub fn kind(&self) -> &EffectKind {
        &self.kind
    }
    pub fn payload(&self) -> &Value {
        &self.payload
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    protocol: String,
    api_version: String,
    request_id: String,
    status: String,
    #[serde(default)]
    output: Value,
    effects: Vec<Effect>,
    #[serde(default)]
    error: Option<Remote>,
}
#[derive(Deserialize)]
struct Remote {
    code: String,
    message: String,
}
#[derive(Debug)]
pub struct Outcome {
    pub(super) plugin: String,
    pub(super) scope: String,
    pub(super) generation: u64,
    output: Value,
    effects: Vec<Effect>,
}
impl Outcome {
    pub fn plugin(&self) -> &str {
        &self.plugin
    }
    pub fn scope(&self) -> &str {
        &self.scope
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn output(&self) -> &Value {
        &self.output
    }
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
    pub(super) fn parse(
        value: Value,
        expected: &str,
        plugin: String,
        scope: String,
        generation: u64,
    ) -> Result<Self, Error> {
        let result: Wire = serde_json::from_value(value).map_err(|_| Error::Wire)?;
        if result.protocol != catalog::PROTOCOL
            || result.api_version != catalog::API
            || result.request_id != expected
            || result.effects.len() > 256
        {
            return Err(Error::Wire);
        }
        if let Some(error) = &result.error {
            if !catalog::text(&error.code, 128) || !catalog::text(&error.message, 4096) {
                return Err(Error::Wire);
            }
        }
        match result.status.as_str() {
            "succeeded" => Ok(Self {
                plugin,
                scope,
                generation,
                output: result.output,
                effects: result.effects,
            }),
            "cancelled" => Err(Error::Cancelled),
            "failed" => match result.error {
                Some(error) => Err(Error::Remote {
                    code: error.code,
                    message: error.message,
                }),
                None => Err(Error::Denied),
            },
            // 该 RPC 交付的是完整结果，不能把 accepted/progress 当成已完成并应用 effects。
            _ => Err(Error::Wire),
        }
    }
}
#[cfg(test)]
#[path = "loom_extension_result_tests.rs"]
mod tests;
