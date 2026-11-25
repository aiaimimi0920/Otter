//! Delivery checks from mount results to local scenes; borrow results only and preserve recovery evidence on rejection.
use super::Outcome;
use crate::{
    device_authorization_lease::{Error as LeaseError, Lease},
    surface_scene::{Error as SceneError, Scene},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    Authorization(LeaseError),
    Unconfirmed,
    Identity,
    Scene(SceneError),
}
/// Scene and revision from the same verified snapshot; lifecycleRevision cannot replace this revision.
#[derive(Debug)]
pub struct AdmittedSnapshot {
    scene: Scene,
    runtime: crate::surface_host_capabilities::Runtime,
    revision: u64,
    resources: crate::surface_resource_snapshot::ResourceSnapshot,
}
impl AdmittedSnapshot {
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) fn into_parts(
        self,
    ) -> (
        Scene,
        u64,
        crate::surface_resource_snapshot::ResourceSnapshot,
        crate::surface_host_capabilities::Runtime,
    ) {
        (self.scene, self.revision, self.resources, self.runtime)
    }
}
impl Outcome {
    /// A delivery-time check, not long-term display permission; handle revocation and expiry throughout the window lifetime.
    pub fn snapshot_for(
        &self,
        lease: &Lease,
        now_ms: u64,
    ) -> Result<AdmittedSnapshot, AdmissionError> {
        lease
            .validate(&self.origin, now_ms)
            .map_err(AdmissionError::Authorization)?;
        if self.uncertain {
            return Err(AdmissionError::Unconfirmed);
        }
        let binding = self.binding.as_ref().ok_or(AdmissionError::Unconfirmed)?;
        let raw = self
            .result
            .as_ref()
            .map_err(|_| AdmissionError::Unconfirmed)?;
        if raw.get().len() > crate::surface_scene::MAX_BYTES {
            return Err(AdmissionError::Scene(SceneError::Limit));
        }
        if self.device != lease.device_id()
            || binding.origin() != self.origin
            || binding.device() != self.device
            || binding.node() != self.request.node()
        {
            return Err(AdmissionError::Identity);
        }
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Header<'a> {
            protocol_version: String,
            instance_id: String,
            attachment_id: String,
            revision: u64,
            #[serde(default)]
            runtime: crate::surface_host_capabilities::Runtime,
            #[serde(borrow)]
            scene: &'a serde_json::value::RawValue,
        }
        let header: Header<'_> =
            serde_json::from_str(raw.get()).map_err(|_| AdmissionError::Identity)?;
        if header.protocol_version != "loom.surface.v1"
            || header.instance_id != binding.instance()
            || header.attachment_id != binding.attachment()
            || header.revision == 0
        {
            return Err(AdmissionError::Identity);
        }
        use crate::surface_host_capabilities::Runtime;
        // JS scenes grant only action authority and prepare hidden HWNDs; never display them as post-execution presentation scenes.
        if header.runtime != Runtime::Declarative
            && !(header.runtime == Runtime::Javascript
                && self.request.supports(Runtime::Javascript))
        {
            return Err(AdmissionError::Scene(SceneError::Runtime));
        }
        // Role and runtime are verified; parse the borrowed scene directly without rescanning the snapshot envelope.
        let scene = Scene::parse(header.scene.get().as_bytes()).map_err(AdmissionError::Scene)?;
        lease
            .validate(&self.origin, now_ms)
            .map_err(AdmissionError::Authorization)?;
        Ok(AdmittedSnapshot {
            scene,
            runtime: header.runtime,
            revision: header.revision,
            resources: crate::surface_resource_snapshot::ResourceSnapshot::new(
                raw,
                binding,
                header.revision,
            )
            .map_err(|_| AdmissionError::Identity)?,
        })
    }
}
