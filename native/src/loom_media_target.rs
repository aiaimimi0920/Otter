//! 清单 HTTP 媒体目标：仅本机可复用清单凭据；远端必须走独立 Device 授权流程。
use crate::live_relay_media_endpoint::{self, Credential, Endpoint, Prepared};
use rustls::RootCertStore;

// 不派生 Debug/Serialize，不公开清单 token。
pub struct Target {
    origin: String,
    token: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    RemoteAuthorizationRequired,
    Endpoint(live_relay_media_endpoint::Error),
    Trust(crate::loom_tls_trust::Error),
}
impl Target {
    pub(super) fn new(origin: String, token: Option<String>) -> Self {
        Self { origin, token }
    }
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn local_viewer_with_trust(
        &self,
        session: &str,
        after: (u64, u64),
        trust: &crate::loom_tls_trust::Trust,
    ) -> Result<Prepared, Error> {
        self.local_viewer(
            session,
            after,
            trust.roots_for(&self.origin).map_err(Error::Trust)?,
        )
    }

    /// 与主干 loopback_surface_authorization 一致，使用固定本机 device ID。
    /// roots 必须是此 origin 的受信配置，清单内容本身不能提供 CA 信任。
    pub fn local_viewer(
        &self,
        session: &str,
        after: (u64, u64),
        roots: RootCertStore,
    ) -> Result<Prepared, Error> {
        let url = url::Url::parse(&self.origin)
            .map_err(|_| Error::Endpoint(live_relay_media_endpoint::Error::Origin))?;
        if !live_relay_media_endpoint::loopback(&url) {
            return Err(Error::RemoteAuthorizationRequired);
        }
        let endpoint = Endpoint::new(&self.origin, roots).map_err(Error::Endpoint)?;
        let credential = self
            .token
            .as_deref()
            .map(Credential::Bearer)
            .unwrap_or(Credential::None);
        endpoint
            .viewer(session, "device-000-local", after, credential)
            .map_err(Error::Endpoint)
    }
}

#[cfg(test)]
#[path = "loom_media_target_tests.rs"]
mod tests;
