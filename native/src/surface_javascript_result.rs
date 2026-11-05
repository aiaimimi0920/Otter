//! 本地 Art 显式结果请求；不是预览或 emit，不接受 URL、文件路径及脚本自报宿主身份。
use super::{encode, invalid};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, io};

pub(crate) const PROTOCOL: &str = "otter.art.result.v1";
const MAX_BYTES: usize = 64 * 1024;
const MAX_PORTS: usize = 128;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Output {
    Value {
        value: Value,
    },
    Canvas {
        #[serde(rename = "nodeId")]
        node_id: String,
    },
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Request {
    pub protocol_version: String,
    #[serde(deserialize_with = "ports")]
    pub outputs: BTreeMap<String, Output>,
}
fn ports<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, Output>, D::Error> {
    struct Visitor;
    impl<'de> serde::de::Visitor<'de> for Visitor {
        type Value = BTreeMap<String, Output>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("bounded unique result ports")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some(name) = map.next_key::<String>()? {
                if result.len() >= MAX_PORTS || !identifier(&name) || result.contains_key(&name) {
                    return Err(serde::de::Error::custom("invalid result port"));
                }
                result.insert(name, map.next_value()?);
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Visitor)
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_:-".contains(&byte))
}
impl Request {
    pub(crate) fn validate(&self) -> io::Result<()> {
        if self.protocol_version != PROTOCOL
            || self.outputs.len() > MAX_PORTS
            || encode(self)?.len() > MAX_BYTES
        {
            return Err(invalid());
        }
        for (name, output) in &self.outputs {
            if !identifier(name)
                || matches!(output, Output::Canvas { node_id } if !identifier(node_id))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    /// 只共享同一已验证帧的像素租约；先验证全部引用，失败不交付半个端口集合。
    pub(crate) fn resolve(
        self,
        images: &BTreeMap<String, crate::image_decode::Image>,
    ) -> io::Result<Resolved> {
        self.validate()?;
        let mut outputs = BTreeMap::new();
        for (name, value) in self.outputs {
            let output = match value {
                Output::Value { value } => ResolvedOutput::Value(value),
                Output::Canvas { node_id } => {
                    ResolvedOutput::Image(images.get(&node_id).ok_or_else(invalid)?.clone())
                }
            };
            outputs.insert(name, output);
        }
        Ok(Resolved { outputs })
    }
}
pub(crate) enum ResolvedOutput {
    Value(Value),
    Image(crate::image_decode::Image),
}
pub(crate) struct Resolved {
    pub outputs: BTreeMap<String, ResolvedOutput>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_result_keeps_empty_and_null_without_host_identity() {
        for outputs in [
            serde_json::json!({}),
            serde_json::json!({"value":{"kind":"value","value":null}}),
        ] {
            let raw = serde_json::json!({"protocolVersion":PROTOCOL,"outputs":outputs});
            let request: Request = serde_json::from_value(raw.clone()).unwrap();
            request.validate().unwrap();
            assert_eq!(serde_json::to_value(request).unwrap(), raw);
        }
        let forged =
            serde_json::json!({"protocolVersion":PROTOCOL,"outputs":{},"workspaceId":"other"});
        assert!(serde_json::from_value::<Request>(forged).is_err());
        assert!(serde_json::from_str::<Request>(r#"{"protocolVersion":"otter.art.result.v1","outputs":{"x":{"kind":"value","value":1},"x":{"kind":"value","value":2}}}"#).is_err());
    }
    #[test]
    fn result_rejects_unknown_version_resource_reference_and_limits() {
        let mut request = Request {
            protocol_version: PROTOCOL.into(),
            outputs: BTreeMap::new(),
        };
        request.protocol_version = "loom.surface.v1".into();
        assert!(request.validate().is_err());
        request.protocol_version = PROTOCOL.into();
        request.outputs.insert(
            "image".into(),
            Output::Canvas {
                node_id: "C:/private.png".into(),
            },
        );
        assert!(request.validate().is_err());
        request.outputs.insert(
            "image".into(),
            Output::Canvas {
                node_id: "canvas:result".into(),
            },
        );
        request.validate().unwrap();
        request.outputs.insert(
            "large".into(),
            Output::Value {
                value: "x".repeat(MAX_BYTES).into(),
            },
        );
        assert!(request.validate().is_err());
        request.outputs = (0..=MAX_PORTS)
            .map(|i| (format!("port{i}"), Output::Value { value: Value::Null }))
            .collect();
        assert!(request.validate().is_err());
        let remote = serde_json::json!({"protocolVersion":PROTOCOL,"outputs":{"image":{"kind":"resource","url":"https://example/image"}}});
        assert!(serde_json::from_value::<Request>(remote).is_err());
    }
}
