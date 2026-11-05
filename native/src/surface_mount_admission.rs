//! 挂载结果到本地场景的交付检查；仅借用结果，拒绝时保留恢复证据。
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
/// 同一已验证快照的场景与版本；不得用 lifecycleRevision 替代此 revision。
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
    /// 这是交付时刻检查，不是长期显示许可；窗口存活期间仍须处理撤销和过期。
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
        struct Header {
            protocol_version: String,
            instance_id: String,
            attachment_id: String,
            revision: u64,
            #[serde(default)]
            runtime: crate::surface_host_capabilities::Runtime,
            scene: Box<serde_json::value::RawValue>,
        }
        let header: Header =
            serde_json::from_str(raw.get()).map_err(|_| AdmissionError::Identity)?;
        if header.protocol_version != "loom.surface.v1"
            || header.instance_id != binding.instance()
            || header.attachment_id != binding.attachment()
            || header.revision == 0
        {
            return Err(AdmissionError::Identity);
        }
        use crate::surface_host_capabilities::Runtime;
        // JS 的 scene 只授予动作权威并准备隐藏 HWND；绝不能作为执行后的呈现场景显示。
        let scene = match header.runtime {
            Runtime::Declarative => crate::live_relay_http::declarative_scene(raw.get()),
            Runtime::Javascript if self.request.supports(Runtime::Javascript) => {
                Scene::parse(header.scene.get().as_bytes())
            }
            _ => Err(SceneError::Runtime),
        }
        .map_err(AdmissionError::Scene)?;
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
