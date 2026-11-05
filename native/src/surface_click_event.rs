//! 声明式输入到有界 wire 事件；编码不是授权，发送 owner 仍须复核租约和当前版本。
use crate::{surface_click_route::ClickAction, surface_live_binding::Binding};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::rand::{SecureRandom, SystemRandom};
use serde::Serialize;
use std::io::{self, Write};

pub const MAX_EVENT_BYTES: usize = 64 * 1024;
#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Modifiers {
    pub alt_key: bool,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Revision,
    Random,
    Encoding,
}
/// 不提供反序列化或可变 body；单次事件重用原字节/ID，不在重试时生成新动作身份。
pub struct ClickEvent {
    id: String,
    bytes: Vec<u8>,
    binding: Binding,
    revision: u64,
    attempted: bool,
    // 仅父侧 JS 权威校验后标记，不进入 wire，也不能由远端响应升级默认 runtime。
    javascript: bool,
}
impl ClickEvent {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn attempted(&self) -> bool {
        self.attempted
    }
    pub(crate) fn is_javascript(&self) -> bool {
        self.javascript
    }
    pub(crate) fn javascript(mut self) -> Self {
        self.javascript = true;
        self
    }
    pub(crate) fn begin_submission(&mut self) {
        self.attempted = true;
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Wire<'a> {
    protocol_version: &'static str,
    instance_id: &'a str,
    attachment_id: &'a str,
    event_id: &'a str,
    node_id: &'a str,
    event: &'a str,
    action: &'a str,
    class: &'a str,
    generation: u64,
    base_revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    modifiers: Option<Modifiers>,
    payload: &'a serde_json::Value,
}
/// 仅用于纯wire编码，不表示普通Surface挂载或Wall输入授权。
#[derive(Clone, Copy)]
pub(crate) struct Identity<'a> {
    pub instance: &'a str,
    pub attachment: &'a str,
    pub generation: u64,
}
impl<'a> From<&'a Binding> for Identity<'a> {
    fn from(binding: &'a Binding) -> Self {
        Self {
            instance: binding.instance(),
            attachment: binding.attachment(),
            generation: binding.generation(),
        }
    }
}
impl ClickAction<'_> {
    /// 无表单调用者保持原 payload；生产宿主使用 encode_with_values。
    pub fn encode(
        &self,
        binding: &Binding,
        revision: u64,
        modifiers: Modifiers,
    ) -> Result<ClickEvent, Error> {
        self.encode_with_values(binding, revision, modifiers, &Default::default())
    }
    pub fn encode_with_values(
        &self,
        binding: &Binding,
        revision: u64,
        modifiers: Modifiers,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<ClickEvent, Error> {
        let (id, bytes) = self.encode_identity(binding.into(), revision, modifiers, values)?;
        Ok(ClickEvent {
            id,
            bytes,
            binding: binding.clone(),
            revision,
            attempted: false,
            javascript: false,
        })
    }
    pub(crate) fn encode_identity(
        &self,
        identity: Identity<'_>,
        revision: u64,
        modifiers: Modifiers,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(String, Vec<u8>), Error> {
        let empty = serde_json::Value::Object(Default::default());
        let payload = self
            .node
            .props()
            .get("eventPayload")
            .filter(|value| !value.is_null())
            .unwrap_or(&empty);
        let merged;
        let payload = if self.node.props().get("includeSurfaceValues")
            == Some(&serde_json::Value::Bool(true))
            && !values.is_empty()
        {
            let mut object = payload.as_object().cloned().unwrap_or_else(|| {
                let mut object = serde_json::Map::new();
                object.insert("configured".to_owned(), payload.clone());
                object
            });
            object.insert(
                "surfaceValues".to_owned(),
                serde_json::Value::Object(values.clone()),
            );
            merged = serde_json::Value::Object(object);
            &merged
        } else {
            payload
        };
        encode_identity(
            identity,
            revision,
            Declaration {
                node_id: self.node.id(),
                event: "click",
                action: self.action,
                class: "discrete",
                modifiers: (self.node.kind() != crate::surface_scene::Kind::Button)
                    .then_some(modifiers),
                payload,
            },
        )
    }
}
/// 各输入类型在其 owner 校验后提供声明，共用不可变 ID/字节及同一个序列化预算。
pub(crate) struct Declaration<'a> {
    pub node_id: &'a str,
    pub event: &'a str,
    pub action: &'a str,
    pub class: &'static str,
    pub modifiers: Option<Modifiers>,
    pub payload: &'a serde_json::Value,
}
pub(crate) fn encode(
    binding: &Binding,
    revision: u64,
    declaration: Declaration<'_>,
) -> Result<ClickEvent, Error> {
    let (id, bytes) = encode_identity(binding.into(), revision, declaration)?;
    Ok(ClickEvent {
        id,
        bytes,
        binding: binding.clone(),
        revision,
        attempted: false,
        javascript: false,
    })
}
pub(crate) fn encode_identity(
    identity: Identity<'_>,
    revision: u64,
    declaration: Declaration<'_>,
) -> Result<(String, Vec<u8>), Error> {
    if revision == 0 {
        return Err(Error::Revision);
    }
    if identity.generation > 9_007_199_254_740_991
        || !crate::live_relay_viewer_identity::identifier(identity.instance)
        || !crate::live_relay_viewer_identity::identifier(identity.attachment)
    {
        return Err(Error::Encoding);
    }
    let mut random = [0_u8; 16];
    SystemRandom::new()
        .fill(&mut random)
        .map_err(|_| Error::Random)?;
    let id = format!("event:{}", URL_SAFE_NO_PAD.encode(random));
    let wire = Wire {
        protocol_version: "loom.surface.v1",
        instance_id: identity.instance,
        attachment_id: identity.attachment,
        event_id: &id,
        node_id: declaration.node_id,
        event: declaration.event,
        action: declaration.action,
        class: declaration.class,
        generation: identity.generation,
        base_revision: revision,
        modifiers: declaration.modifiers,
        payload: declaration.payload,
    };
    let mut output = Bounded(Vec::new());
    serde_json::to_writer(&mut output, &wire).map_err(|_| Error::Encoding)?;
    Ok((id, output.0))
}
struct Bounded(Vec<u8>);
impl Write for Bounded {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_EVENT_BYTES.saturating_sub(self.0.len()) {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let needed = self.0.len() + bytes.len();
        if needed > self.0.capacity() {
            // 有界几何增长，避免 serializer 的小块写入触发逐字节重分配。
            let capacity = needed
                .max(self.0.capacity().saturating_mul(2))
                .clamp(512, MAX_EVENT_BYTES);
            self.0
                .try_reserve_exact(capacity - self.0.len())
                .map_err(|_| io::ErrorKind::OutOfMemory)?;
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "surface_click_event_tests.rs"]
mod tests;
