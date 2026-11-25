//! Account projection borrows the same device lease; no token copying or promotion of local Art leases to account permissions.
use super::*;
impl Lease {
    /// Background only. The currently connected Loom decides account permissions; share links cannot change transport targets.
    pub fn account_projections(
        &self,
        _discovery: &crate::loom_discovery::Config,
    ) -> Result<crate::projection_account::Account<'_>, crate::projection_account::Error> {
        let session = self
            .online_session()
            .map_err(|_| crate::projection_account::Error::Credential)?;
        let config = crate::device_authorization_config::Config::from_env(session.origin())
            .map_err(|_| crate::projection_account::Error::Credential)?;
        let client = config.prepare_client(&self.state.cancel).map_err(|_| {
            if self.state.cancel.is_stopped() {
                crate::projection_account::Error::Stopped
            } else {
                crate::projection_account::Error::Http
            }
        })?;
        crate::projection_account::Account::authorized(client, session, &self.state.cancel)
    }
}
