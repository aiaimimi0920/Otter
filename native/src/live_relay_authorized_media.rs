//! 媒体启动只从授权租约取得 token，沿用控制 Client 的固定 origin、TLS 和解析地址。
use super::*;
use crate::{
    byte_budget::Budget,
    live_relay_media_endpoint::{Credential, Error as EndpointError},
    live_relay_viewer_runtime::Runtime,
    live_relay_viewer_session::Permit,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Credential,
    Clock,
    Endpoint(EndpointError),
    Start,
    Unbound,
}
impl Lease {
    pub(crate) fn source_media(
        &self,
        client: &Client,
        session: &str,
    ) -> Result<crate::live_relay_media_endpoint::Prepared, Error> {
        let now = crate::live_relay_source_runtime::now().map_err(|_| Error::Clock)?;
        self.validate(client.origin(), now)
            .map_err(|_| Error::Stopped)?;
        let token = self
            .online_session()
            .map_err(|_| Error::Credential)?
            .token_for(client.origin(), now)
            .map_err(|_| Error::Credential)?;
        client
            .media_endpoint()
            .source(session, self.device_id(), Credential::Device(token))
            .map_err(Error::Endpoint)
    }
    /// 每次重建都重新验证同一Grant/固定origin/到期及owner，签发新nonce；不做HTTP或续签。
    fn viewer_media(
        &self,
        client: &Client,
        session: &str,
        media: &Arc<ViewerSession>,
        permit: &Permit,
        after: u64,
    ) -> Result<crate::live_relay_media_endpoint::Prepared, Error> {
        if self.is_revoked() {
            return Err(Error::Stopped);
        }
        let bound = self
            .state
            .media
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|bound| Arc::ptr_eq(&bound, media));
        if !bound {
            return Err(Error::Unbound);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Clock)?
            .as_millis();
        let now = u64::try_from(now).map_err(|_| Error::Clock)?;
        media.validate(permit).map_err(|_| Error::Unbound)?;
        let token = self
            .online_session()
            .map_err(|_| Error::Credential)?
            .token_for(client.origin(), now)
            .map_err(|_| Error::Credential)?;
        let prepared = client
            .media_endpoint()
            .viewer(
                session,
                self.device_id(),
                (permit.epoch(), after),
                Credential::Device(token),
            )
            .map_err(Error::Endpoint)?;
        if self.is_revoked() {
            return Err(Error::Stopped);
        }
        Ok(prepared)
    }
    /// 直接调用者保留单次连接合同；产品控制owner使用下方同身份恢复入口。
    #[cfg(test)]
    pub(crate) fn start_media(
        &self,
        client: &Client,
        session: &str,
        media: Arc<ViewerSession>,
        permit: Permit,
        budget: Arc<Budget>,
        completed: impl FnOnce(crate::live_relay_viewer_runtime::Exit) + Send + 'static,
    ) -> Result<Runtime, Error> {
        let prepared = self.viewer_media(client, session, &media, &permit, 0)?;
        let runtime = Runtime::start_observed(
            prepared,
            client.addresses(),
            media,
            permit,
            budget,
            completed,
        )
        .map_err(|_| Error::Start)?;
        if self.is_revoked() {
            runtime.request_stop();
            return Err(Error::Stopped);
        }
        Ok(runtime)
    }
    pub(crate) fn start_media_recovering(
        self: &Arc<Self>,
        client: &Client,
        media: Arc<ViewerSession>,
        permit: Permit,
        budget: Arc<Budget>,
        input: Arc<crate::live_relay_control_runtime::interaction::Queue>,
        completed: impl FnOnce(crate::live_relay_viewer_runtime::Exit) + Send + 'static,
    ) -> Result<Runtime, Error> {
        let session = media.identity().session().to_owned();
        let prepared = self.viewer_media(client, &session, &media, &permit, 0)?;
        let addresses = client.addresses();
        let (lease, client, bound, authority) =
            (self.clone(), client.clone(), media.clone(), permit.clone());
        let notify = input.clone();
        let recovery = crate::live_relay_viewer_runtime::Recovery::new(
            permit.epoch(),
            move |cursor| {
                lease
                    .viewer_media(&client, &session, &bound, &authority, cursor.frame_id)
                    .map_err(|_| ())
            },
            move |recovering| notify.media_recovering(recovering),
        );
        let runtime = Runtime::start_recovering(
            prepared,
            addresses,
            media,
            permit,
            budget,
            Some(recovery),
            completed,
        )
        .map_err(|_| Error::Start)?;
        #[cfg(test)]
        input.test_bind_disconnect(runtime.test_disconnect());
        Ok(runtime)
    }
}
