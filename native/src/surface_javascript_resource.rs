//! JS 入口只能取自当前快照的 binary/application/javascript 租约；复用既有 TLS/摘要门。
use super::*;
use crate::{surface_host_capabilities::Runtime, surface_live_binding::Binding};
pub(crate) struct Entry(ResourceLease);
pub(crate) struct Source {
    pub entry: Entry,
    pub bytes: Zeroizing<Vec<u8>>,
}
pub(crate) fn same_attachment(old: &Binding, next: &Binding) -> bool {
    old.origin() == next.origin()
        && old.device() == next.device()
        && old.node() == next.node()
        && old.instance() == next.instance()
        && old.attachment() == next.attachment()
        && old.generation() == next.generation()
        && old.lifecycle_revision() <= next.lifecycle_revision()
        && (old.lifecycle_revision() != next.lifecycle_revision()
            || old.lifecycle() == next.lifecycle())
}
impl MountedSurface {
    pub(crate) fn javascript_entry(&self, now: u64) -> Result<Entry, ResourceError> {
        if self.snapshot.get().len() > crate::surface_scene::MAX_BYTES {
            return Err(ResourceError::Limit);
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Header {
            runtime: Runtime,
            entry_resource_id: String,
        }
        let header: Header =
            serde_json::from_str(self.snapshot.get()).map_err(|_| ResourceError::Input)?;
        if header.runtime != Runtime::Javascript {
            return Err(ResourceError::Unsupported);
        }
        let entry = Entry(self.resource_request(&header.entry_resource_id)?);
        entry.validate(now)?;
        Ok(entry)
    }
}
fn validate(request: &ResourceLease, now: u64) -> Result<(), ResourceError> {
    request.validate_common(now)?;
    let r = &request.lease.resource;
    if r.kind != "binary"
        || r.mime != "application/javascript"
        || r.width.is_some()
        || r.height.is_some()
    {
        return Err(ResourceError::Unsupported);
    }
    if request.size() > crate::surface_javascript_wire::MAX_SOURCE {
        return Err(ResourceError::Limit);
    }
    Ok(())
}
impl Entry {
    pub(crate) fn validate(&self, now: u64) -> Result<(), ResourceError> {
        validate(&self.0, now)
    }
    pub(crate) fn binding(&self) -> &Binding {
        self.0.binding()
    }
    pub(crate) fn revision(&self) -> u64 {
        self.0.revision()
    }
    pub(crate) fn expires_at_ms(&self) -> u64 {
        self.0.expires_at_ms()
    }
    /// 相同源码可随显式新快照更新资源租约；禁止静默换程序、设备或实例代际。
    pub(crate) fn current(
        &self,
        mount: &MountedSurface,
        now: u64,
        update: bool,
    ) -> Result<Entry, ResourceError> {
        // 新租约不能复活早已过期的执行状态；必须重新获取/挂载一份新会话。
        self.validate(now)?;
        let next = mount.javascript_entry(now)?;
        if !same_attachment(self.binding(), next.binding())
            || self.0.lease.resource != next.0.lease.resource
            || if update {
                next.revision() <= self.revision()
            } else {
                next.revision() != self.revision() || next.0.lease != self.0.lease
            }
        {
            return Err(ResourceError::Identity);
        }
        Ok(next)
    }
}
impl Control<'_> {
    pub(crate) fn fetch_javascript_entry(&self, entry: Entry) -> Result<Source, ResourceError> {
        let verified = self.fetch_resource(entry.0, validate)?;
        let (request, bytes) = verified.into_parts();
        let source = std::str::from_utf8(&bytes).map_err(|_| ResourceError::Input)?;
        if source.contains('\0') {
            return Err(ResourceError::Input);
        }
        Ok(Source {
            entry: Entry(request),
            bytes,
        })
    }
}
