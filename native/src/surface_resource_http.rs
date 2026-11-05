//! 经 mount 身份约束的图片字节获取。这里只验证传输，不解码、不缓存、不发布 UI。
use super::*;
use crate::surface_live_binding::Binding;
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceError {
    Input,
    Identity,
    Missing,
    Limit,
    Expired,
    Unsupported,
    Digest,
    Http(Error),
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Descriptor {
    resource_id: String,
    kind: String,
    mime: String,
    size: usize,
    width: Option<u32>,
    height: Option<u32>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct Transport {
    kind: String,
    path: Option<String>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Lease {
    lease_id: String,
    resource: Descriptor,
    transport: Transport,
    expires_at_ms: u64,
}
/// 字段私有；只能从当前挂载的受限快照取得，不能构造任意 URL 或头。
pub struct ResourceLease {
    binding: Binding,
    revision: u64,
    lease: Lease,
}
pub struct VerifiedResource {
    request: ResourceLease,
    bytes: Zeroizing<Vec<u8>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    protocol_version: String,
    instance_id: String,
    attachment_id: String,
    revision: u64,
    #[serde(default)]
    resource_leases: Vec<Lease>,
}
impl MountedSurface {
    pub fn image_resource(&self, id: &str, now: u64) -> Result<ResourceLease, ResourceError> {
        let request = self.resource_request(id)?;
        request.validate(now)?;
        Ok(request)
    }
    fn resource_request(&self, id: &str) -> Result<ResourceLease, ResourceError> {
        if self.snapshot.get().len() > 1024 * 1024 {
            return Err(ResourceError::Limit);
        }
        let snapshot: Snapshot =
            serde_json::from_str(self.snapshot.get()).map_err(|_| ResourceError::Input)?;
        if snapshot.protocol_version != "loom.surface.v1"
            || snapshot.instance_id != self.binding.instance()
            || snapshot.attachment_id != self.binding.attachment()
            || snapshot.revision == 0
        {
            return Err(ResourceError::Identity);
        }
        if snapshot.resource_leases.len() > 64 {
            return Err(ResourceError::Limit);
        }
        let mut leases = snapshot
            .resource_leases
            .into_iter()
            .filter(|lease| lease.resource.resource_id == id);
        let lease = leases.next().ok_or(ResourceError::Missing)?;
        if leases.next().is_some() {
            return Err(ResourceError::Identity);
        }
        Ok(ResourceLease {
            binding: self.binding.clone(),
            revision: snapshot.revision,
            lease,
        })
    }
}
impl ResourceLease {
    pub(crate) fn expires_at_ms(&self) -> u64 {
        self.lease.expires_at_ms
    }
    pub(crate) fn mime(&self) -> &str {
        &self.lease.resource.mime
    }
    pub(crate) fn dimensions(&self) -> Option<(u32, u32)> {
        self.lease.resource.width.zip(self.lease.resource.height)
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn resource_id(&self) -> &str {
        &self.lease.resource.resource_id
    }
    pub(crate) fn id(&self) -> &str {
        &self.lease.lease_id
    }
    pub(crate) fn size(&self) -> usize {
        self.lease.resource.size
    }
    pub(crate) fn path(&self) -> &str {
        self.lease.transport.path.as_deref().unwrap_or("")
    }
    pub(crate) fn validate(&self, now: u64) -> Result<(), ResourceError> {
        self.validate_common(now)?;
        let r = &self.lease.resource;
        if r.kind != "image" {
            return Err(ResourceError::Unsupported);
        }
        if r.mime == "application/x-neuro-rgba8" {
            let pixels = r
                .width
                .zip(r.height)
                .filter(|(w, h)| *w > 0 && *h > 0)
                .and_then(|(w, h)| (w as usize).checked_mul(h as usize))
                .and_then(|n| n.checked_mul(4));
            if pixels != Some(self.size()) {
                return Err(ResourceError::Input);
            }
        }
        Ok(())
    }
    fn validate_common(&self, now: u64) -> Result<(), ResourceError> {
        let lease = &self.lease;
        if lease.expires_at_ms <= now || lease.expires_at_ms > now.saturating_add(3_600_000) {
            return Err(ResourceError::Expired);
        }
        let r = &lease.resource;
        let digest = r
            .resource_id
            .strip_prefix("sha256:")
            .ok_or(ResourceError::Input)?;
        if digest.len() != 64
            || !digest.bytes().all(|b| b.is_ascii_hexdigit())
            || self.id().is_empty()
            || self.id().len() > 160
            || !self
                .id()
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b))
            || r.mime.trim().is_empty()
            || r.mime.len() > 160
            || !r.mime.is_ascii()
        {
            return Err(ResourceError::Input);
        }
        if self.size() == 0 || self.size() > 16 * 1024 * 1024 {
            return Err(ResourceError::Limit);
        }
        // shared_memory/stream 留给各自安全 owner，不降级为路径读取或网络访问。
        if lease.transport.kind != "loom_resource" {
            return Err(ResourceError::Unsupported);
        }
        if self.path() != format!("/v1/surfaces/resources/{digest}") {
            return Err(ResourceError::Input);
        }
        Ok(())
    }
    pub(crate) fn matches(&self, current: &MountedSurface, now: u64) -> Result<(), ResourceError> {
        let next = current.image_resource(&self.lease.resource.resource_id, now)?;
        if next.binding != self.binding
            || next.revision != self.revision
            || next.lease != self.lease
        {
            return Err(ResourceError::Identity);
        }
        Ok(())
    }
}
impl VerifiedResource {
    pub(crate) fn into_parts(self) -> (ResourceLease, Zeroizing<Vec<u8>>) {
        (self.request, self.bytes)
    }
    /// 消费前必须提交当前 mount；换代/租约替换/到期的迟到结果不可取出字节。
    pub fn bytes<'a>(
        &'a self,
        current: &MountedSurface,
        now: u64,
        cancel: &Cancel,
    ) -> Result<&'a [u8], ResourceError> {
        if cancel.is_stopped() {
            return Err(ResourceError::Http(Error::Http(
                crate::device_session_client::Error::Stopped,
            )));
        }
        self.request.matches(current, now)?;
        Ok(&self.bytes)
    }
    pub fn mime(&self) -> &str {
        &self.request.lease.resource.mime
    }
}
impl Control<'_> {
    /// 阻塞方法只供后台 owner 使用；复用固定地址 TLS、取消、截止时间与 Device 401 语义。
    pub fn fetch_image_resource(
        &self,
        request: ResourceLease,
    ) -> Result<VerifiedResource, ResourceError> {
        self.fetch_resource(request, ResourceLease::validate)
    }
    fn fetch_resource(
        &self,
        request: ResourceLease,
        validate: impl Fn(&ResourceLease, u64) -> Result<(), ResourceError>,
    ) -> Result<VerifiedResource, ResourceError> {
        let clock = || now().map_err(ResourceError::Http);
        validate(&request, clock()?)?;
        if request.binding.origin() != self.client.origin()
            || request.binding.device() != self.session.device_id()
        {
            return Err(ResourceError::Identity);
        }
        let bytes = self
            .send_resource_until(
                "GET",
                request.path(),
                &[],
                Instant::now() + Duration::from_secs(20),
                request.size(),
                Some(&request),
            )
            .map_err(ResourceError::Http)?;
        validate(&request, clock()?)?;
        if bytes.len() != request.size() {
            return Err(ResourceError::Limit);
        }
        let digest = format!("{:x}", Sha256::digest(&bytes));
        if !digest.eq_ignore_ascii_case(&request.lease.resource.resource_id[7..]) {
            return Err(ResourceError::Digest);
        }
        if self.cancel.is_stopped() {
            return Err(ResourceError::Http(Error::Http(
                crate::device_session_client::Error::Stopped,
            )));
        }
        Ok(VerifiedResource { request, bytes })
    }
}
#[path = "surface_javascript_resource.rs"]
pub(crate) mod javascript;
