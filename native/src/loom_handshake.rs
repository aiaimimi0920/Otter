//! 原生能力握手数据边界；只保留有界目录，不自动执行能力、不输出远端数据日志。
use serde::Deserialize;
use serde_json::Value;
pub const MAX_BYTES: usize = 1024 * 1024;
#[derive(Debug, PartialEq)]
pub enum Error {
    TooLarge,
    Encoding,
    Remote,
    Protocol,
    Identity,
    Transport,
    Capabilities,
}
#[derive(Clone, Copy, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Websocket,
    SharedMemory,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Handshake {
    protocol_version: String,
    server_name: String,
    server_version: String,
    session_id: String,
    transport: Transport,
    capabilities: Capabilities,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Capabilities {
    art_definitions: Vec<Value>,
    operations: Vec<String>,
    surface: Value,
}
impl Handshake {
    pub fn arts(&self) -> &[Value] {
        &self.capabilities.art_definitions
    }
    pub fn capability(&self, index: usize) -> Option<crate::loom_capability::Capability<'_>> {
        crate::loom_capability::Capability::new(self.arts().get(index)?)
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn transport(&self) -> Transport {
        self.transport
    }
    pub fn server_name(&self) -> &str {
        &self.server_name
    }
    pub fn server_version(&self) -> &str {
        &self.server_version
    }
    pub fn operations(&self) -> &[String] {
        &self.capabilities.operations
    }
    pub fn surface(&self) -> &Value {
        &self.capabilities.surface
    }
}
fn text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
pub fn request(client_id: &str, version: &str) -> Result<String, Error> {
    if !text(client_id) || !text(version) {
        return Err(Error::Identity);
    }
    Ok(serde_json::json!({"method":"loom.hook.handshake","params":{
        "protocolVersion":"loom.hook.v1","supportedProtocolVersions":["loom.hook.v1"],
        "clientId":client_id,"clientVersion":version,"platform":"windows","deviceId":"device:local",
        "transports":["websocket","shared_memory"]}})
    .to_string())
}
/// method 事件不作为握手响应；其余格式/错误拒绝，避免用无效响应替换已展示目录。
pub fn response(bytes: &[u8]) -> Result<Option<Handshake>, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    let envelope: Value = serde_json::from_slice(bytes).map_err(|_| Error::Encoding)?;
    if envelope.get("method").is_some() {
        return Ok(None);
    }
    if envelope.get("error").is_some_and(|error| !error.is_null()) {
        return Err(Error::Remote);
    }
    // 再从原文解码，保留 serde 对重复已知字段的拒绝，不经 Value 覆盖后再解码。
    let result: Handshake = serde_json::from_slice(bytes).map_err(|_| Error::Encoding)?;
    if result.protocol_version != "loom.hook.v1" {
        return Err(Error::Protocol);
    }
    if !text(&result.server_name) || !text(&result.server_version) || !text(&result.session_id) {
        return Err(Error::Identity);
    }
    let caps = &result.capabilities;
    if caps.art_definitions.len() > 512
        || caps.operations.len() > 512
        || caps.operations.iter().any(|value| !text(value))
    {
        return Err(Error::Capabilities);
    }
    for name in ["runtimes", "nodes", "transports", "capabilities"] {
        let entries = caps.surface[name].as_array().ok_or(Error::Capabilities)?;
        if entries.len() > 512
            || entries
                .iter()
                .any(|entry| !entry.as_str().is_some_and(text))
        {
            return Err(Error::Capabilities);
        }
    }
    if !caps.surface["apiVersion"].as_str().is_some_and(text)
        || ["pointer", "hover", "touch", "keyboard"]
            .iter()
            .any(|name| !caps.surface["input"][name].is_boolean())
    {
        return Err(Error::Capabilities);
    }
    for (index, art) in caps.art_definitions.iter().enumerate() {
        let id = art["id"]
            .as_str()
            .filter(|id| text(id))
            .ok_or(Error::Capabilities)?;
        if caps.art_definitions[..index].iter().any(|other| {
            other["id"]
                .as_str()
                .is_some_and(|other| other.trim() == id.trim())
        }) {
            return Err(Error::Capabilities);
        }
        // 与主干一致，缺失/空 label 回退 id；有效标签仍执行文本预算和控制字符检查。
        if art["label"]
            .as_str()
            .is_some_and(|label| !label.trim().is_empty() && !text(label))
        {
            return Err(Error::Capabilities);
        }
        for name in ["parameters", "inputs", "outputs"] {
            let entries = art[name].as_array().ok_or(Error::Capabilities)?;
            if entries.len() > 128 || entries.iter().any(|entry| !entry.is_object()) {
                return Err(Error::Capabilities);
            }
        }
    }
    Ok(Some(result))
}
#[cfg(test)]
#[path = "loom_handshake_tests.rs"]
pub(crate) mod tests;
