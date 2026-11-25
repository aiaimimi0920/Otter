//! Select current device/node Live bindings from authenticated mount responses; parsing is not authorization and creates no replacement IDs.
use crate::live_relay_viewer_identity::identifier;
use serde::Deserialize;
use std::collections::BTreeMap;
const MAX_RESPONSE: usize = 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Input,
    Limit,
    Json,
    Missing,
    Ambiguous,
    Identity,
    Disposed,
    Revision,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    origin: String,
    device: String,
    node: String,
    instance: String,
    attachment: String,
    generation: u64,
    lifecycle_revision: u64,
    lifecycle: Lifecycle,
}
#[derive(Deserialize)]
struct Mounted {
    instance: Instance,
}
#[derive(Deserialize)]
struct Instance {
    descriptor: Descriptor,
    attachments: BTreeMap<String, Attachment>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    instance_id: String,
    generation: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Attachment {
    descriptor: AttachmentDescriptor,
    lifecycle: Lifecycle,
    lifecycle_revision: u64,
}
#[derive(Clone, Copy, Debug, Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Created,
    Mounted,
    Active,
    Inactive,
    Suspended,
    Disposed,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AttachmentDescriptor {
    attachment_id: String,
    instance_id: String,
    hook_node_id: String,
    device_id: String,
}
impl Binding {
    /// Host view for new local Art without fabricated remote attach/ACK; this value alone grants no JS execution or service permissions.
    pub(crate) fn local_art(
        request: &crate::art_execution_grant::wire::Binding,
        node: &str,
        attachment: &str,
    ) -> Result<Self, Error> {
        let origin = crate::device_session_attempt::canonical_origin(&request.origin)
            .map_err(|_| Error::Input)?;
        if origin != request.origin
            || [&request.device_id, &request.instance_id, node, attachment]
                .iter()
                .any(|value| !identifier(value))
            || request.generation == 0
            || request.generation > 9_007_199_254_740_991
        {
            return Err(Error::Identity);
        }
        Ok(Self {
            origin,
            device: request.device_id.clone(),
            node: node.into(),
            instance: request.instance_id.clone(),
            attachment: attachment.into(),
            generation: request.generation,
            lifecycle_revision: 1,
            lifecycle: Lifecycle::Active,
        })
    }
    pub(crate) fn acknowledge(
        &self,
        bytes: &[u8],
        state: Lifecycle,
        revision: u64,
    ) -> Result<Self, Error> {
        if bytes.len() > MAX_RESPONSE {
            return Err(Error::Limit);
        }
        let ack: Attachment = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        let d = ack.descriptor;
        if d.instance_id != self.instance
            || d.attachment_id != self.attachment
            || d.device_id != self.device
            || d.hook_node_id != self.node
            || ack.lifecycle != state
        {
            return Err(Error::Identity);
        }
        if revision != self.next_lifecycle_revision()? || ack.lifecycle_revision != revision {
            return Err(Error::Revision);
        }
        let mut result = self.clone();
        result.lifecycle_revision = revision;
        result.lifecycle = state;
        Ok(result)
    }
    /// origin/device must come from this HTTP request's pinned Client and authorization lease, not response declarations.
    pub fn from_mount(bytes: &[u8], origin: &str, device: &str, node: &str) -> Result<Self, Error> {
        let origin =
            crate::device_session_attempt::canonical_origin(origin).map_err(|_| Error::Input)?;
        if !identifier(device) || !identifier(node) {
            return Err(Error::Input);
        }
        if bytes.is_empty() || bytes.len() > MAX_RESPONSE {
            return Err(Error::Limit);
        }
        let mounted: Mounted = serde_json::from_slice(bytes).map_err(|_| Error::Json)?;
        let instance = mounted.instance;
        if !identifier(&instance.descriptor.instance_id) {
            return Err(Error::Identity);
        }
        let mut selected = None;
        for (id, attachment) in instance.attachments {
            let descriptor = attachment.descriptor;
            // Shared instances may retain old device bindings; do not select the first entry solely by node.
            if descriptor.device_id != device || descriptor.hook_node_id != node {
                continue;
            }
            if selected.is_some() {
                return Err(Error::Ambiguous);
            }
            if !identifier(&id)
                || descriptor.attachment_id != id
                || descriptor.instance_id != instance.descriptor.instance_id
            {
                return Err(Error::Identity);
            }
            if attachment.lifecycle == Lifecycle::Disposed {
                return Err(Error::Disposed);
            }
            selected = Some((id, attachment.lifecycle_revision, attachment.lifecycle));
        }
        let (attachment, lifecycle_revision, lifecycle) = selected.ok_or(Error::Missing)?;
        Ok(Self {
            origin,
            device: device.into(),
            node: node.into(),
            instance: instance.descriptor.instance_id,
            attachment,
            generation: instance.descriptor.generation,
            lifecycle_revision,
            lifecycle,
        })
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn device(&self) -> &str {
        &self.device
    }
    pub fn node(&self) -> &str {
        &self.node
    }
    pub fn instance(&self) -> &str {
        &self.instance
    }
    pub fn attachment(&self) -> &str {
        &self.attachment
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn lifecycle_revision(&self) -> u64 {
        self.lifecycle_revision
    }
    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
    }
    /// Explicit initial-view target; later discovery still rechecks catalog identity, and this cannot be used for automatic renewal.
    pub fn viewer_target(
        &self,
        selected: &crate::live_relay_viewer_identity::Identity,
        origin: &str,
        device: &str,
        node: &str,
        generation: u64,
    ) -> Result<crate::live_relay_control_runtime::Target, Error> {
        if !self.matches(origin, device, node, generation) {
            return Err(Error::Identity);
        }
        Ok(crate::live_relay_control_runtime::Target {
            session: selected.session().into(),
            surface: self.instance.clone(),
            attachment: self.attachment.clone(),
            expected: None,
            selected: Some(selected.clone()),
        })
    }
    pub fn next_lifecycle_revision(&self) -> Result<u64, Error> {
        self.lifecycle_revision
            .checked_add(1)
            .ok_or(Error::Revision)
    }
    /// Recheck context before UI use; live server validation remains necessary, and caches are not permanent authorization.
    pub fn matches(&self, origin: &str, device: &str, node: &str, generation: u64) -> bool {
        self.origin == origin
            && self.device == device
            && self.node == node
            && self.generation == generation
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-live")))]
#[path = "surface_live_binding_tests.rs"]
mod tests;
