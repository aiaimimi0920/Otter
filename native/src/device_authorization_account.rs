//! 账号投射借用同一设备租约；不复制 token，不把本地 Art 租约升级为账号权限。
use super::*;
impl Lease {
    /// 仅在后台调用。当前已连接 Loom 裁决账号权限，分享链接不能切换传输目标。
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
