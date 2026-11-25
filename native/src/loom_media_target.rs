//! Manifest HTTP media targets: only local connections reuse manifest credentials; remote targets require separate Device authorization.
use crate::live_relay_media_endpoint::{self, Credential, Endpoint, Prepared};
use rustls::RootCertStore;

// Do not derive Debug/Serialize or expose manifest tokens.
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

    /// Only for local discovery-manifest management channels; never give these credentials to LAN or projection sources.
    pub(crate) fn local_control(&self) -> Result<(&str, &str), Error> {
        let url = url::Url::parse(&self.origin).map_err(|_| Error::RemoteAuthorizationRequired)?;
        if url.scheme() != "http" || !live_relay_media_endpoint::loopback(&url) {
            return Err(Error::RemoteAuthorizationRequired);
        }
        Ok((
            &self.origin,
            self.token
                .as_deref()
                .ok_or(Error::RemoteAuthorizationRequired)?,
        ))
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

    /// Match mainline loopback_surface_authorization using the fixed local device ID.
    /// roots must be trusted configuration for this origin; manifest contents themselves cannot establish CA trust.
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

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "loom_media_target_tests.rs"]
mod tests;
