//! Device-scoped queries for uncertain initial mounts; candidates are neither creation receipts nor holders of lifecycle revisions.
use super::*;
use crate::surface_live_binding::Error as BindingError;
use serde::Deserialize;

#[derive(Debug, PartialEq, Eq)]
pub struct RecoveryCandidate {
    origin: String,
    device: String,
    art: String,
    node: String,
    instance: String,
    attachment: String,
    generation: u64,
}
impl RecoveryCandidate {
    #[cfg(test)]
    pub(crate) fn fixture() -> Self {
        Self {
            origin: "https://localhost:1234".into(),
            device: "viewer:test".into(),
            art: "art:test".into(),
            node: "node:test".into(),
            instance: "instance:test".into(),
            attachment: "attachment:test".into(),
            generation: 1,
        }
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
    pub fn art(&self) -> &str {
        &self.art
    }
    pub fn node(&self) -> &str {
        &self.node
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }
    pub fn device(&self) -> &str {
        &self.device
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stream {
    protocol_version: String,
    next: u64,
    reset: bool,
    messages: Vec<Message>,
}
#[derive(Deserialize)]
struct Message {
    method: String,
    params: Box<serde_json::value::RawValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Params {
    hook_node_id: String,
    generation: u64,
    snapshot: SnapshotHeader,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotHeader {
    protocol_version: String,
    instance_id: String,
    attachment_id: String,
    art_id: String,
    revision: u64,
}
impl Control<'_> {
    /// Remount only after explicit acceptance of the current candidate. It may have remote side effects, is not retried automatically, and does not prove original creation succeeded.
    pub fn remount_surface_candidate(
        &self,
        candidate: &RecoveryCandidate,
        report: &crate::surface_host_capabilities::Report,
    ) -> Result<MountedSurface, Error> {
        if candidate.origin != self.client.origin() || candidate.device != self.session.device_id()
        {
            return Err(Error::SurfaceBinding(BindingError::Identity));
        }
        let mut url = url::Url::parse("https://path.invalid/").map_err(|_| Error::Input)?;
        url.path_segments_mut()
            .map_err(|_| Error::Input)?
            .clear()
            .extend(["v1", "surfaces", "instances", &candidate.instance, "mount"]);
        let body = serde_json::to_vec(&serde_json::json!({"attachmentId":candidate.attachment}))
            .map_err(|_| Error::Input)?;
        let bytes = self.send("POST", url.path(), &body, Duration::from_secs(10))?;
        let binding = crate::surface_live_binding::Binding::from_mount(
            &bytes,
            &candidate.origin,
            &candidate.device,
            &candidate.node,
        )
        .map_err(Error::SurfaceBinding)?;
        if binding.instance() != candidate.instance
            || binding.attachment() != candidate.attachment
            || binding.generation() < candidate.generation
        {
            return Err(Error::SurfaceBinding(BindingError::Identity));
        }
        let mounted = surface::snapshot(&bytes, binding)?;
        surface_attach::validate(&bytes, &mounted, &candidate.art, report)?;
        Ok(mounted)
    }
    /// Background-only. None does not prove absence of creation: created but unattached instances are absent from recovery snapshots.
    pub fn surface_recovery_candidate(
        &self,
        art: &str,
        node: &str,
    ) -> Result<Option<RecoveryCandidate>, Error> {
        if !protocol::identifier(art) || !protocol::identifier(node) {
            return Err(Error::Input);
        }
        let bytes = self.send(
            "GET",
            "/v1/surfaces/stream?after=0&timeoutMs=0",
            &[],
            Duration::from_secs(10),
        )?;
        candidate(
            &bytes,
            self.client.origin(),
            self.session.device_id(),
            art,
            node,
        )
    }
}
fn candidate(
    bytes: &[u8],
    origin: &str,
    device: &str,
    art: &str,
    node: &str,
) -> Result<Option<RecoveryCandidate>, Error> {
    let stream: Stream = protocol::parse(bytes).map_err(Error::Protocol)?;
    if stream.protocol_version != "loom.surface-stream.v1" || stream.messages.len() > 512 {
        return Err(Error::SurfaceBinding(BindingError::Limit));
    }
    // This query finds only current bindings and establishes no incremental cursor; reset/next must still have correct wire types.
    let _ = (stream.next, stream.reset);
    let mut found = None;
    for message in stream.messages {
        if message.method != "loom.surface.snapshot" {
            continue;
        }
        let params: Params = serde_json::from_str(message.params.get())
            .map_err(|_| Error::SurfaceBinding(BindingError::Json))?;
        if params.hook_node_id != node {
            continue;
        }
        let snapshot = params.snapshot;
        if snapshot.protocol_version != "loom.surface.v1"
            || snapshot.art_id != art
            || !protocol::identifier(&snapshot.instance_id)
            || !protocol::identifier(&snapshot.attachment_id)
            || snapshot.revision == 0
            || matches!(snapshot.instance_id.as_str(), "." | "..")
        {
            return Err(Error::SurfaceBinding(BindingError::Identity));
        }
        let current = RecoveryCandidate {
            origin: origin.into(),
            device: device.into(),
            art: art.into(),
            node: node.into(),
            instance: snapshot.instance_id,
            attachment: snapshot.attachment_id,
            generation: params.generation,
        };
        // Recovery snapshots and broadcasts may duplicate entries; collapse only matching identities without guessing order across different candidates.
        if found.as_ref().is_some_and(|previous| previous != &current) {
            return Err(Error::SurfaceBinding(BindingError::Ambiguous));
        }
        found = Some(current);
    }
    Ok(found)
}
