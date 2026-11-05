//! Tile控制、图片/媒体与Art各持同Grant独立串行Lease；UI独立探测输出/超时清屏。
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
        // 并行HTTP必须独立租约且来自同一Grant，不能借另一个设备/新凭据延长旧会话。
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
    /// 输出重绑定只沿原Grant；被服务撤销/过期的旧owner不能借几何变化重新启动。
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
