//! Tile control, image/media, and Art each own independent serial Leases from the same Grant; UI independently probes outputs and clears on timeout.
use crate::{
    device_authorization_config::Config, device_authorization_lease::Lease, tile_assets::Assets,
    tile_outputs::TileOutput, tile_shared::Shared,
};
use std::{sync::Arc, thread::JoinHandle, time::Duration};
#[path = "tile_presenter.rs"]
mod presenter;
pub struct Runtime {
    pub(crate) shared: Arc<Shared>,
    lease: Arc<Lease>,
    images: Arc<Lease>,
    art: Arc<Lease>,
    thread: Option<JoinHandle<()>>,
}
impl Runtime {
    pub fn start(
        config: Config,
        lease: Lease,
        images: Lease,
        art: Lease,
        output: TileOutput,
    ) -> std::io::Result<Self> {
        Self::spawn(lease, images, art, output, move |token| {
            token.prepare_client(&config).map_err(|_| "连接准备失败")
        })
    }
    #[cfg(test)]
    pub(crate) fn test_start(
        client: crate::device_session_client::Client,
        lease: Lease,
        images: Lease,
        art: Lease,
        output: TileOutput,
    ) -> std::io::Result<Self> {
        Self::spawn(lease, images, art, output, move |_| Ok(client))
    }
    fn spawn(
        lease: Lease,
        images: Lease,
        art: Lease,
        output: TileOutput,
        prepare: impl FnOnce(&Lease) -> Result<crate::device_session_client::Client, &'static str>
            + Send
            + 'static,
    ) -> std::io::Result<Self> {
        // Parallel HTTP requires separate leases from the same Grant; another device/new credentials cannot extend an old session.
        if !lease.same_grant(&images) || !lease.same_grant(&art) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Tile lease grant mismatch",
            ));
        }
        let lease = Arc::new(lease);
        let images = Arc::new(images);
        let art = Arc::new(art);
        let shared = Arc::new(Shared::default());
        let revoked = shared.clone();
        lease.notify_revocation(Arc::new(move || revoked.stop()));
        let revoked = shared.clone();
        art.notify_revocation(Arc::new(move || revoked.stop()));
        let (token, image_token, state) = (lease.clone(), images.clone(), shared.clone());
        let art_token = art.clone();
        let thread = std::thread::Builder::new()
            .name("otter-tile-presenter".into())
            .spawn(move || {
                let run = || -> Result<(), &'static str> {
                    let client = prepare(&token)?;
                    let assets =
                        Assets::start(client.clone(), image_token, art_token, state.clone())
                            .map_err(|_| "图片线程启动失败")?;
                    presenter::run(&client, &token, &state, &assets, output)
                };
                if let Err(reason) = run() {
                    state.lock().clear(reason);
                }
            })?;
        Ok(Self {
            shared,
            lease,
            images,
            art,
            thread: Some(thread),
        })
    }
    pub fn request_stop(&self) {
        self.shared.stop();
        self.images.revoke();
        self.art.revoke();
    }
    pub fn finished(&self) -> bool {
        self.thread.as_ref().is_none_or(|t| t.is_finished())
    }
    pub fn invalidate_output(&self) {
        self.shared.stop();
        self.images.revoke();
        self.art.revoke();
    }
    pub fn status(&self) -> &'static str {
        self.shared.lock().status
    }
    /// Output rebinding uses only the original Grant; server-revoked/expired owners cannot restart through geometry changes.
    pub(crate) fn authorization_valid(&self, origin: &str) -> bool {
        self.lease.validate(origin, now_ms()).is_ok()
    }
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.lease.revoke();
    }
}
pub(crate) fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| d.as_millis().try_into().ok())
        .unwrap_or(u64::MAX)
}
