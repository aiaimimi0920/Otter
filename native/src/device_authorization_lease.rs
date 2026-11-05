//! 单个串行后台请求链的凭据租约；缓存替换/撤销同步取消请求并关闭绑定媒体。
use crate::{
    device_request_cancel::Cancel, device_session_attempt::Session, device_session_client::Client,
    live_relay_http::Control, live_relay_viewer_session::ViewerSession,
};
use std::sync::{Arc, Mutex, Weak};
#[path = "live_relay_authorized_media.rs"]
mod media;
pub use media::Error as MediaError;
#[path = "device_authorization_watch.rs"]
mod watch;
pub use watch::Notification;
#[path = "device_authorization_execution.rs"]
mod execution;
#[path = "device_authorization_local.rs"]
mod local;
#[path = "tile_live_authorization.rs"]
mod tile;

const MAX_LEASES: usize = 16;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Revoked,
    Credential,
    Capacity,
    Bound,
}

// Session 不复制 token。最后一个后台租约退出后，由 Session 的 Zeroizing 擦除。
pub(super) struct Grant {
    session: Arc<Session>,
    leases: Vec<Weak<State>>,
}
struct State {
    cancel: Cancel,
    execution_cancel: Cancel,
    execution_watch: watch::Watch,
    media: Mutex<Option<Weak<ViewerSession>>>,
    watch: watch::Watch,
}
pub struct Lease {
    session: Option<Arc<Session>>,
    local: Option<Box<local::Credential>>,
    state: Arc<State>,
}
impl Grant {
    pub(super) fn new(session: Session) -> Self {
        Self {
            session: Arc::new(session),
            leases: Vec::new(),
        }
    }
    pub(super) fn token_for(&self, origin: &str, now: u64) -> Option<&str> {
        self.session.token_for(origin, now).ok()
    }
    pub(super) fn lease(&mut self, origin: &str, now: u64) -> Result<Lease, Error> {
        self.session
            .token_for(origin, now)
            .map_err(|_| Error::Credential)?;
        self.leases.retain(|lease| lease.strong_count() != 0);
        if self.leases.len() >= MAX_LEASES {
            return Err(Error::Capacity);
        }
        let state = Arc::new(State {
            cancel: Cancel::new(),
            execution_cancel: Cancel::new(),
            execution_watch: watch::Watch::default(),
            media: Mutex::new(None),
            watch: watch::Watch::default(),
        });
        self.leases.push(Arc::downgrade(&state));
        Ok(Lease {
            session: Some(self.session.clone()),
            local: None,
            state,
        })
    }
}
impl Drop for Grant {
    fn drop(&mut self) {
        for state in self.leases.iter().filter_map(Weak::upgrade) {
            state.revoke();
        }
    }
}
impl State {
    fn revoke(&self) {
        // 与绑定共用锁；先关闭媒体，再唤醒网络，不等待线程 join。
        let media = self.media.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(media) = media.as_ref().and_then(Weak::upgrade) {
            media.close();
        }
        self.cancel.cancel();
        self.execution_cancel.cancel();
        self.execution_watch.revoke();
        self.watch.revoke();
    }
}
impl Lease {
    #[cfg(test)]
    pub(crate) fn test_disconnect_request(&self) -> bool {
        self.state.cancel.test_disconnect()
    }
    /// 惰性创建独立手动重置事件，撤销后永久有信号；不占网络取消的唤醒槽。
    pub fn revocation_notification(&self) -> std::io::Result<Notification> {
        self.state.watch.notification()
    }
    /// 原串行链的长期结果复用宿主 Wake；注册与撤销原子衔接，不新增等待句柄。
    pub(crate) fn notify_revocation(&self, notify: Arc<dyn Fn() + Send + Sync>) {
        self.state.watch.notify(notify);
    }
    /// 仅用于安排授权截止等待；不是 token，也不能替代 validate。
    pub fn expires_at_ms(&self) -> u64 {
        self.session
            .as_ref()
            .map(|s| s.expires_at_ms())
            .unwrap_or_else(|| self.local.as_ref().unwrap().expires())
    }
    pub fn device_id(&self) -> &str {
        self.session
            .as_ref()
            .map(|s| s.device_id())
            .unwrap_or_else(|| self.local.as_ref().unwrap().device())
    }
    pub fn is_revoked(&self) -> bool {
        self.state.cancel.is_stopped()
    }
    /// 后台有界编码共用原请求停止标志，不能用独立的 false 标志绕过撤权。
    pub(crate) fn preparation_cancel(&self) -> &std::sync::atomic::AtomicBool {
        self.state.cancel.flag()
    }
    /// 已完成资源仅能移交给同一 Grant 的长期 owner；相同 origin/device 不足以证明同一凭据代际。
    pub(crate) fn same_grant(&self, other: &Self) -> bool {
        match (&self.session, &other.session) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => Arc::ptr_eq(&self.state, &other.state),
            _ => false,
        }
    }
    /// 本地交付前复核，不返回或复制 token；调用方必须传入当前时间。
    pub fn validate(&self, origin: &str, now_ms: u64) -> Result<(), Error> {
        if self.is_revoked() {
            return Err(Error::Revoked);
        }
        match &self.session {
            Some(session) => {
                session
                    .token_for(origin, now_ms)
                    .map_err(|_| Error::Credential)?;
            }
            None => {
                if let Err(error) = self
                    .local
                    .as_ref()
                    .ok_or(Error::Credential)?
                    .validate(origin, now_ms)
                {
                    self.revoke();
                    return Err(error);
                }
            }
        }
        if self.is_revoked() {
            return Err(Error::Revoked);
        }
        Ok(())
    }
    /// 每个租约只属于一个串行请求链；并行连接必须从缓存申请独立租约。
    /// Control 在每次请求前后另行检查 origin/过期；取消信号覆盖借用后发生的撤销。
    pub fn control<'a>(&'a self, client: &'a Client) -> Result<Control<'a>, Error> {
        if self.is_revoked() {
            return Err(Error::Revoked);
        }
        Ok(Control::new(
            client,
            self.online_session()?,
            &self.state.cancel,
        ))
    }
    /// 图片结果持有原请求租约，消费不得换成另一个未取消的 Cancel 绕过撤权。
    pub(crate) fn validate_resource(
        &self,
        resource: &crate::live_relay_http::ResourceLease,
        current: &crate::live_relay_http::MountedSurface,
        now: u64,
    ) -> Result<(), crate::live_relay_http::ResourceError> {
        use crate::live_relay_http::ResourceError;
        self.validate(current.binding.origin(), now)
            .map_err(|_| ResourceError::Identity)?;
        if current.binding.device() != self.device_id() {
            return Err(ResourceError::Identity);
        }
        resource.matches(current, now)
    }
    /// 一租约只绑定一个媒体 owner；终态 close 防止撤销与后续 activate 交错复活。
    pub fn bind_media(&self, media: &Arc<ViewerSession>) -> Result<(), Error> {
        let mut slot = self.state.media.lock().unwrap_or_else(|e| e.into_inner());
        if self.is_revoked() {
            media.close();
            return Err(Error::Revoked);
        }
        if slot.is_some() {
            return Err(Error::Bound);
        }
        *slot = Some(Arc::downgrade(media));
        Ok(())
    }
    pub fn revoke(&self) {
        self.state.revoke();
    }
    pub(super) fn wait(&self, duration: std::time::Duration) -> bool {
        self.state.cancel.wait(duration)
    }
    /// 配置快照到 Client 的准备也属于此串行租约，撤销可唤醒原生 DNS 等待。
    pub(super) fn prepare_client(
        &self,
        config: &crate::device_authorization_config::Config,
    ) -> Result<Client, crate::device_authorization_config::Error> {
        use crate::device_authorization_config::Error as PrepareError;
        let validate = || {
            if self.is_revoked() {
                return Err(PrepareError::Stopped);
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| PrepareError::Clock)?
                .as_millis();
            let now = u64::try_from(now).map_err(|_| PrepareError::Clock)?;
            self.online_session()
                .map_err(|_| PrepareError::Credential)?
                .token_for(config.origin(), now)
                .map_err(|_| PrepareError::Credential)?;
            Ok(())
        };
        validate()?;
        let client = config.prepare_client(&self.state.cancel)?;
        validate()?;
        Ok(client)
    }
    fn online_session(&self) -> Result<&Session, Error> {
        self.session.as_deref().ok_or(Error::Credential)
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.revoke();
    }
}

#[cfg(test)]
#[path = "device_authorization_lease_tests.rs"]
mod tests;
