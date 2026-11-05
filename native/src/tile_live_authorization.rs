//! 同一串行Tile Assets租约；新握手重新核原Grant并取nonce，不自动续签。
use super::*;
impl Lease {
    pub(crate) fn wall_media(
        &self,
        client: &Client,
        key: &crate::tile_shared::Key,
        session: &str,
        format: crate::tile_live_packet::WallMediaFormat,
    ) -> Result<crate::live_relay_media_endpoint::Prepared, MediaError> {
        let now = crate::tile_runtime::now_ms();
        self.validate(client.origin(), now)
            .map_err(|_| MediaError::Stopped)?;
        let token = self
            .online_session()
            .map_err(|_| MediaError::Credential)?
            .token_for(client.origin(), now)
            .map_err(|_| MediaError::Credential)?;
        client
            .media_endpoint()
            .wall(key, session, token, self.device_id(), format)
            .map_err(MediaError::Endpoint)
    }
}
