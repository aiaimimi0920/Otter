//! 已释放 Loom 正式主端口到原样 Shader 内容；不扫描次选端口、不加载内容指示的资源。
use super::loom_output_batch::{Batch, Output};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
#[path = "shader_payload_decode.rs"]
mod decode;

#[derive(Debug, PartialEq)]
pub enum Error {
    Unsupported,
    Source,
    Bindings,
    Budget,
    Cancelled,
    Decode,
    Dimensions,
    Admission,
}
#[derive(Deserialize)]
pub(super) struct Payload {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    pub(super) abi: Option<String>,
    #[serde(default, deserialize_with = "decode::source")]
    pub source: String,
    #[serde(default, deserialize_with = "decode::source")]
    pub vertex_shader: String,
    #[serde(default, deserialize_with = "decode::source")]
    pub fragment_shader: String,
    #[serde(default, deserialize_with = "decode::uniforms")]
    pub uniforms: BTreeMap<String, f32>,
    #[serde(default, deserialize_with = "decode::textures")]
    pub textures: BTreeMap<String, String>,
    success: Option<bool>,
}
impl Payload {
    pub fn is_native(&self) -> bool {
        self.abi.as_deref() == Some(crate::shader_hlsl::ABI)
    }
    pub fn from_batch(batch: Batch) -> Result<Self, Error> {
        let (mut ports, primary) = batch.into_ports();
        let Output::Value(value) = ports.swap_remove(primary).value else {
            return Err(Error::Unsupported);
        };
        Self::from_value(value)
    }
    fn from_value(value: Value) -> Result<Self, Error> {
        let payload: Self = if value["type"].as_str() == Some("shader") {
            // direct Value 已由协议解析；类型化前先拒绝超量，避免再构造一份大映射。
            if value["textures"]
                .as_object()
                .is_some_and(|map| map.len() > 31)
                || value["uniforms"]
                    .as_object()
                    .is_some_and(|map| map.len() > 256)
            {
                return Err(Error::Bindings);
            }
            for field in ["vertex_shader", "fragment_shader", "source"] {
                if value[field]
                    .as_str()
                    .is_some_and(|text| !decode::valid_source(text))
                {
                    return Err(Error::Source);
                }
            }
            serde_json::from_value(value).map_err(|_| Error::Unsupported)?
        } else {
            let text = value["content"]
                .get(0)
                .and_then(|first| first["text"].as_str())
                .filter(|text| text.len() <= 64 * 1024 * 1024)
                .ok_or(Error::Unsupported)?;
            // 直接反序列化有计数上限的字段，不先将 wrapper 文本展开为另一个无界 Value 树。
            serde_json::from_str(text).map_err(|_| Error::Unsupported)?
        };
        if payload.kind != "shader" || payload.success == Some(false) {
            return Err(Error::Unsupported);
        }
        let sources = if payload.is_native() {
            if !payload.vertex_shader.is_empty() || !payload.fragment_shader.is_empty() {
                return Err(Error::Unsupported);
            }
            vec![&payload.source]
        } else {
            if payload.abi.is_some() || !payload.source.is_empty() {
                return Err(Error::Unsupported);
            }
            vec![&payload.vertex_shader, &payload.fragment_shader]
        };
        for source in sources {
            if source.is_empty()
                || source.len() > crate::shader_worker_wire::MAX_SOURCE
                || source.contains('\0')
            {
                return Err(Error::Source);
            }
        }
        if payload.textures.len() > 31 || payload.uniforms.len() > 256 {
            return Err(Error::Bindings);
        }
        for (name, value) in &payload.uniforms {
            if crate::shader_angle::name(name).is_err() || !value.is_finite() {
                return Err(Error::Bindings);
            }
        }
        for name in payload.textures.keys() {
            if name == "input" || crate::shader_angle::name(name).is_err() {
                // 旧异步 response.input 与外部 input 有竞争；新链拒绝歧义，不静默择一。
                return Err(Error::Bindings);
            }
        }
        Ok(payload)
    }
}
