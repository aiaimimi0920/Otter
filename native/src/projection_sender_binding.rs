//! 兼容 projectionSenders 的来源关联；候选/索引不是授权、磁盘提交或接收端呈现凭证。
use super::*;
use crate::{session_json_fields::fields, session_source::Source};
#[path = "projection_sender_admission.rs"]
mod admission;
#[path = "projection_sender_checkpoint.rs"]
pub mod checkpoint;
#[path = "projection_sender_binding_inventory.rs"]
mod inventory;
#[path = "projection_sender_binding_patch.rs"]
mod patch;
#[path = "projection_sender_binding_route.rs"]
mod route_binding;
#[path = "projection_sender_binding_stop.rs"]
pub(crate) mod stop;
pub const MAX_PER_UNIT: usize = 8;
pub(super) const MAX_BINDING_BYTES: usize = 16 * 1024;
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    link: Link,
    target: Target,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Link {
    role: String,
    local_unit_id: String,
    envelope: Envelope,
    revision: u64,
    digest: String,
    linked: bool,
    otter_native_send: Marker,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    stop_pending: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    stopped: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    offline_transport: Option<route_binding::Origin>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Marker {
    version: u32,
    scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Target {
    device_id: String,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    peer_id: Option<String>,
}
pub struct Candidate {
    expected: Source,
    source: Source,
    binding: Binding,
    index: checkpoint::Index,
}
impl Candidate {
    pub fn expected(&self) -> &Source {
        &self.expected
    }
    pub fn source(&self) -> &Source {
        &self.source
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn index(&self) -> &checkpoint::Index {
        &self.index
    }
}
impl Binding {
    pub fn unit(&self) -> &str {
        &self.link.local_unit_id
    }
    pub fn envelope(&self) -> &Envelope {
        &self.link.envelope
    }
    pub fn target(&self) -> &str {
        self.link
            .otter_native_send
            .target
            .as_deref()
            .unwrap_or(&self.target.device_id)
    }
    pub fn name(&self) -> &str {
        &self.target.name
    }
    pub fn revision(&self) -> u64 {
        self.link.revision
    }
    pub fn digest(&self) -> &str {
        &self.link.digest
    }
    pub fn linked(&self) -> bool {
        self.link.linked
    }
    pub fn stopped(&self) -> bool {
        self.link.stopped || self.link.stop_pending
    }
    pub fn scope(&self) -> &str {
        &self.link.otter_native_send.scope
    }
    pub fn matches_intent(&self, intent: &Intent) -> bool {
        self.unit() == intent.unit()
            && self.target() == intent.target()
            && self.envelope() == intent.envelope()
            && self.link.otter_native_send.scope == intent.record.selection.scope
            && self.destination() == intent.record.selection.destination()
    }
    fn valid(&self, unit: &str, scope: &str) -> bool {
        self.link.role == "source"
            && self.unit() == unit
            && self.envelope().source_unit() == unit
            && self.envelope().validate(self.envelope().origin()).is_ok()
            && self.link.otter_native_send.version == 1
            && self.link.otter_native_send.scope == scope
            && envelope::digest(scope)
            && envelope::identifier(self.target())
            && self.route_valid()
            && self.target.name.len() <= 1024
            && !self.target.name.chars().any(char::is_control)
            && (self.envelope().source_revision()..=envelope::MAX_REVISION)
                .contains(&self.revision())
            && envelope::digest(self.digest())
            && (self.revision() != self.envelope().source_revision()
                || self.digest() == self.envelope().content_digest())
    }
    fn decode(raw: &str, unit: &str, scope: &str) -> Result<Self, Error> {
        if raw.len() > MAX_BINDING_BYTES {
            return Err(Error::Limit);
        }
        let object = fields(raw).map_err(|_| Error::Format)?;
        let link =
            fields(object.get("link").ok_or(Error::Format)?.get()).map_err(|_| Error::Format)?;
        let target =
            fields(object.get("target").ok_or(Error::Format)?.get()).map_err(|_| Error::Format)?;
        // 明确成对且非null；未知/残缺离线字段仍不能降为在线资格。
        if link.contains_key("offlineTransport") != target.contains_key("peerId")
            || link
                .get("offlineTransport")
                .is_some_and(|v| v.get() == "null")
            || target.get("peerId").is_some_and(|v| v.get() == "null")
        {
            return Err(Error::Format);
        }
        fields(link.get("otterNativeSend").ok_or(Error::Format)?.get())
            .map_err(|_| Error::Format)?;
        let binding: Self = serde_json::from_str(raw).map_err(|_| Error::Format)?;
        if !binding.valid(unit, scope) {
            return Err(Error::Format);
        }
        Ok(binding)
    }
    pub(crate) fn prepare(
        source: &Source,
        intent: &Intent,
        response: &Response,
        name: &str,
        cancel: &AtomicBool,
    ) -> Result<Candidate, Error> {
        check(cancel)?;
        if name.len() > 1024 || name.chars().any(char::is_control) {
            return Err(Error::Limit);
        }
        let selection = &intent.record.selection;
        let offline = selection.offline.as_ref();
        let next = Self {
            link: Link {
                role: "source".into(),
                local_unit_id: intent.unit().into(),
                envelope: intent.envelope().clone(),
                revision: response.revision(),
                digest: response.digest().into(),
                linked: response.receiver_device().is_some(),
                otter_native_send: Marker {
                    version: 1,
                    scope: selection.scope.clone(),
                    target: offline.map(|_| intent.target().into()),
                },
                stop_pending: false,
                stopped: false,
                offline_transport: offline.map(|_| route_binding::Origin {
                    origin: intent.envelope().origin().into(),
                }),
            },
            target: Target {
                device_id: intent.receiver().into(),
                name: name.into(),
                peer_id: offline.map(|r| r.peer_id.clone()),
            },
        };
        if !next.valid(intent.unit(), &selection.scope) {
            return Err(Error::Format);
        }
        let decoded = crate::session_document::decode(source).map_err(|_| Error::Format)?;
        if decoded
            .projection_scope
            .map(|scope| scope.to_string())
            .as_deref()
            != Some(selection.scope.as_str())
        {
            return Err(Error::Changed);
        }
        let patched = patch::prepare(source, &next, cancel)?;
        let index = checkpoint::Index::capture(&patched).ok_or(Error::Format)?;
        if !index.includes(&next) {
            return Err(Error::Changed);
        }
        check(cancel)?;
        Ok(Candidate {
            expected: source.clone(),
            source: patched,
            binding: next,
            index,
        })
    }
}
impl std::fmt::Debug for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionSenderBinding")
            .field("revision", &self.revision())
            .field("linked", &self.linked())
            .field("stopped", &self.stopped())
            .finish_non_exhaustive()
    }
}
#[cfg(test)]
#[path = "projection_sender_binding_tests.rs"]
pub(crate) mod tests;
