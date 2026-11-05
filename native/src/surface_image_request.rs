//! 图片来源的类型化身份；资源分支仍完全委托原远端租约合同。
use crate::{
    device_authorization_lease::Lease,
    live_relay_http::{MountedSurface, ResourceError, ResourceLease},
    surface_live_binding::Binding,
};
#[path = "surface_image_data.rs"]
pub(crate) mod data;
#[path = "surface_image_inline.rs"]
pub mod inline;
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    Resource(String),
    Node(String),
}
pub enum Request {
    Resource(ResourceLease),
    Inline(inline::Inline),
}
impl Request {
    pub fn as_resource(&self) -> Option<&ResourceLease> {
        match self {
            Self::Resource(request) => Some(request),
            Self::Inline(_) => None,
        }
    }
    pub fn key(&self) -> Key {
        match self {
            Self::Resource(request) => Key::Resource(request.resource_id().to_owned()),
            Self::Inline(request) => request.key(),
        }
    }
    pub fn binding(&self) -> &Binding {
        match self {
            Self::Resource(request) => request.binding(),
            Self::Inline(request) => request.binding(),
        }
    }
    pub fn revision(&self) -> u64 {
        match self {
            Self::Resource(request) => request.revision(),
            Self::Inline(request) => request.revision(),
        }
    }
    pub fn expires_at_ms(&self) -> u64 {
        match self {
            Self::Resource(request) => request.expires_at_ms(),
            Self::Inline(_) => u64::MAX, // 只受真实 Grant 截止约束，没有伪造资源租约。
        }
    }
    pub fn mime(&self) -> &str {
        match self {
            Self::Resource(request) => request.mime(),
            Self::Inline(request) => request.mime(),
        }
    }
    pub fn validate(
        &self,
        lease: &Lease,
        current: &MountedSurface,
        now: u64,
    ) -> Result<(), ResourceError> {
        let digest = matches!(self, Self::Inline(_)).then(|| inline::digest(current));
        self.validate_with_digest(lease, current, now, digest.as_ref())
    }
    pub(crate) fn validate_with_digest(
        &self,
        lease: &Lease,
        current: &MountedSurface,
        now: u64,
        digest: Option<&[u8; 32]>,
    ) -> Result<(), ResourceError> {
        match self {
            Self::Resource(request) => lease.validate_resource(request, current, now),
            Self::Inline(request) => {
                lease
                    .validate(current.binding.origin(), now)
                    .map_err(|_| ResourceError::Identity)?;
                if current.binding.device() != lease.device_id() {
                    return Err(ResourceError::Identity);
                }
                request.matches_digest(current, digest.ok_or(ResourceError::Identity)?)
            }
        }
    }
}
