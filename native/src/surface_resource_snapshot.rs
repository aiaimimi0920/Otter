//! 当前呈现场景的不可变资源快照；允许同附件生命周期推进，不允许换设备/代际/场景。
use crate::surface_live_binding::Binding;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Input,
    Identity,
    Limit,
}
#[cfg(windows)]
use crate::live_relay_http::MountedSurface;
use serde::Deserialize;
use serde_json::value::RawValue;
pub struct ResourceSnapshot {
    raw: Box<RawValue>,
    binding: Binding,
    revision: u64,
}
impl std::fmt::Debug for ResourceSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceSnapshot")
            .field("revision", &self.revision)
            .field("bytes", &self.raw.get().len())
            .finish_non_exhaustive()
    }
}
impl ResourceSnapshot {
    /// 只验证快照身份与大小，不授予来源读权；调用方仍须执行授权与完整收敛检查。
    pub(crate) fn new(raw: &RawValue, binding: &Binding, revision: u64) -> Result<Self, Error> {
        if raw.get().len() > crate::surface_scene::MAX_BYTES {
            return Err(Error::Limit);
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Header {
            protocol_version: String,
            instance_id: String,
            attachment_id: String,
            revision: u64,
        }
        let header: Header = serde_json::from_str(raw.get()).map_err(|_| Error::Input)?;
        if revision == 0
            || header.revision != revision
            || header.protocol_version != "loom.surface.v1"
            || header.instance_id != binding.instance()
            || header.attachment_id != binding.attachment()
        {
            return Err(Error::Identity);
        }
        Ok(Self {
            raw: raw.to_owned(),
            binding: binding.clone(),
            revision,
        })
    }
    pub fn validate(&self, current: &Binding, revision: u64) -> Result<(), Error> {
        let old = &self.binding;
        if revision != self.revision
            || current.origin() != old.origin()
            || current.device() != old.device()
            || current.node() != old.node()
            || current.instance() != old.instance()
            || current.attachment() != old.attachment()
            || current.generation() != old.generation()
            || current.lifecycle_revision() < old.lifecycle_revision()
        {
            return Err(Error::Identity);
        }
        Ok(())
    }
    #[cfg(windows)]
    pub(crate) fn mount(&self, current: &Binding, revision: u64) -> Result<MountedSurface, Error> {
        self.validate(current, revision)?;
        Ok(MountedSurface {
            binding: current.clone(),
            snapshot: self.raw.clone(),
        })
    }
    #[cfg(all(test, windows))]
    pub(crate) fn empty_for_test(binding: &Binding, revision: u64) -> Self {
        let raw = serde_json::value::to_raw_value(&serde_json::json!({
            "protocolVersion":"loom.surface.v1","instanceId":binding.instance(),
            "attachmentId":binding.attachment(),"revision":revision,"resourceLeases":[]
        }))
        .unwrap();
        Self::new(&raw, binding, revision).unwrap()
    }
}
#[cfg(test)]
#[path = "surface_resource_snapshot_tests.rs"]
pub(crate) mod tests;
