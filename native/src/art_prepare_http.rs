//! 已授权设备获取精确本地 Art；复用固定 origin HTTP、原租约取消及真实 signed child。
//! 不建立 issuer 信任，不缓存授权；准备结果持有原租约，不能换新凭据绕过撤销。
use super::{now, Client, Control};
use crate::{
    art_execution_grant::{self as grant, wire, Expected, Issuer, Permit},
    device_authorization_lease::Lease,
    surface_javascript::{signed, Frame},
    surface_live_binding::Binding,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
#[path = "art_prepare_cache.rs"]
mod cache;
#[path = "art_prepare_package.rs"]
mod package;
#[path = "art_prepare_resume.rs"]
mod resume;
#[path = "art_service_http.rs"]
pub mod service;
#[path = "art_live_viewer_http.rs"]
pub(crate) mod viewer;

const MAX_SOURCE: usize = crate::art_source_cache::MAX_SOURCE;
const MAX_RESPONSE: usize = 4 * MAX_SOURCE.div_ceil(3) + wire::MAX_ENVELOPE + 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Input,
    Wire,
    Limit,
    Lease,
    Http(super::Error),
    Grant(grant::Error),
    Mount(signed::Error),
}
/// 来自宿主请求 owner，而不是响应；本类型不自动证明 workspace 存在或防止重复请求。
#[derive(Serialize)]
pub struct Request {
    pub binding: wire::Binding,
    pub permissions: Vec<wire::Permission>,
    pub services: Vec<String>,
}
impl Request {
    fn expected(&self) -> Expected<'_> {
        Expected {
            binding: &self.binding,
            permissions: &self.permissions,
            services: &self.services,
        }
    }
}
/// 只能由已认证配置/配对 owner 提供，不从 issuer 路由响应自动建立。
#[derive(Clone)]
pub struct Pin {
    pub key_id: String,
    pub public_key: [u8; 32],
}
impl Pin {
    fn issuer(&self) -> Issuer<'_> {
        Issuer {
            key_id: &self.key_id,
            public_key: &self.public_key,
        }
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    protocol_version: String,
    grant: wire::Envelope,
    source: String,
}
pub struct Prepared {
    request: Request,
    pin: Pin,
    envelope: Vec<u8>,
    source: Vec<u8>,
    lease: Arc<Lease>,
    // 保留第一次准入的单调时间和不可逆状态，不能靠延迟 mount 重新开始期限。
    permit: Arc<Mutex<Permit>>,
    client: Option<Client>,
    resumed: bool,
    assets: Vec<crate::art_package_content::Asset>,
    pub(crate) ports: Option<grant::package::ports::Ports>,
    pub(crate) visibility: Option<grant::package::visibility::Visibility>,
}
impl Prepared {
    pub(crate) fn online(&self) -> bool {
        !self.resumed
    }
    /// 仅暴露此前宿主请求的绑定，供挂载 owner 建立本地 view；不是从响应反推身份。
    pub fn binding(&self) -> &wire::Binding {
        &self.request.binding
    }
    pub(crate) fn lease(&self) -> Arc<Lease> {
        self.lease.clone()
    }
    pub(crate) fn deadline(&self, online: bool) -> (u64, std::time::Instant) {
        let (wall, monotonic) = self
            .permit
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .deadline(online);
        (wall.min(self.lease.expires_at_ms()), monotonic)
    }
    /// 后台串行调用；同一 lease 不允许同时承担其他请求。撤销直接中断既有网络链。
    pub(crate) fn fetch(
        client: &Client,
        lease: Arc<Lease>,
        request: Request,
        pin: Pin,
    ) -> Result<Self, Error> {
        if request.binding.origin != client.origin()
            || request.binding.device_id != lease.device_id()
        {
            return Err(Error::Input);
        }
        lease
            .validate(client.origin(), now().map_err(Error::Http)?)
            .map_err(|_| Error::Lease)?;
        let body = serde_json::to_vec(&request).map_err(|_| Error::Input)?;
        if body.len() > 16 * 1024 {
            return Err(Error::Limit);
        }
        let control = lease.control(client).map_err(|_| Error::Lease)?;
        let bytes = control.art_prepare_response(&body)?;
        let mut prepared = Self::accept(&bytes, lease, request, pin)?;
        prepared.client = Some(client.clone());
        Ok(prepared)
    }
    fn accept(bytes: &[u8], lease: Arc<Lease>, request: Request, pin: Pin) -> Result<Self, Error> {
        if bytes.len() > MAX_RESPONSE {
            return Err(Error::Limit);
        }
        let response: Response = serde_json::from_slice(bytes).map_err(|_| Error::Wire)?;
        if response.protocol_version != "loom.art.local.prepare.v1" {
            return Err(Error::Wire);
        }
        if response.source.len() > 4 * MAX_SOURCE.div_ceil(3) {
            return Err(Error::Limit);
        }
        let source = STANDARD.decode(response.source).map_err(|_| Error::Wire)?;
        let envelope = serde_json::to_vec(&response.grant).map_err(|_| Error::Wire)?;
        Self::accept_parts(envelope, source, lease, request, pin)
    }
    fn accept_parts(
        envelope: Vec<u8>,
        source: Vec<u8>,
        lease: Arc<Lease>,
        request: Request,
        pin: Pin,
    ) -> Result<Self, Error> {
        let time = now().map_err(Error::Http)?;
        lease
            .validate(&request.binding.origin, time)
            .map_err(|_| Error::Lease)?;
        let permit = Permit::accept(&envelope, pin.issuer(), request.expected(), &source, time)
            .map_err(Error::Grant)?;
        lease
            .validate(&request.binding.origin, now().map_err(Error::Http)?)
            .map_err(|_| Error::Lease)?;
        Ok(Self {
            request,
            pin,
            envelope,
            source,
            lease,
            permit: Arc::new(Mutex::new(permit)),
            client: None,
            resumed: false,
            assets: Vec::new(),
            ports: None,
            visibility: None,
        })
    }
    /// 消费准备结果，仅原请求可挂载一次；调用方仍负责生命周期取消和当前 view 身份。
    pub fn mount(
        self,
        view: Binding,
        snapshot: serde_json::Value,
        online: bool,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(signed::Local, Frame), Error> {
        self.mount_in(view, snapshot, online, None, cancelled)
    }
    /// 视口只来自宿主已创建的 HWND，不从远端响应推导或猜测 DPI。
    pub(crate) fn mount_in(
        self,
        view: Binding,
        snapshot: serde_json::Value,
        online: bool,
        viewport: Option<crate::surface_viewport::Viewport>,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(signed::Local, Frame), Error> {
        self.mount_observed(view, snapshot, online, viewport, None, cancelled)
    }
    pub(crate) fn mount_observed(
        self,
        view: Binding,
        snapshot: serde_json::Value,
        online: bool,
        viewport: Option<crate::surface_viewport::Viewport>,
        progress: Option<Arc<service::progress::Channel>>,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(signed::Local, Frame), Error> {
        self.mount_content_observed(
            view,
            signed::runtime::Content::empty(snapshot),
            online,
            viewport,
            progress,
            cancelled,
        )
    }
    pub(crate) fn mount_content_observed(
        self,
        view: Binding,
        content: signed::runtime::Content,
        online: bool,
        viewport: Option<crate::surface_viewport::Viewport>,
        progress: Option<Arc<service::progress::Channel>>,
        cancelled: &impl Fn() -> bool,
    ) -> Result<(signed::Local, Frame), Error> {
        self.permit
            .lock()
            .map_err(|_| Error::Mount(signed::Error::Stopped))?
            .authorize(
                &self.request.binding,
                now().map_err(Error::Http)?,
                online,
                self.lease.is_revoked() || cancelled(),
            )
            .map_err(Error::Grant)?;
        let content =
            crate::art_package_content::merge(content, &self.assets).map_err(|_| Error::Limit)?;
        let dispatcher = service::dispatcher::prepare(&self, online, progress, &content.images)?;
        let input = signed::Input {
            envelope: &self.envelope,
            issuer: self.pin.issuer(),
            expected: self.request.expected(),
            source: &self.source,
            view,
            snapshot: content.snapshot,
            resources: content.resources,
            images: content.images,
            viewport,
        };
        if self.resumed {
            if online {
                return Err(Error::Input);
            }
            return signed::Local::mount_resumed(
                input,
                self.lease,
                cancelled,
                self.permit,
                dispatcher,
            )
            .map_err(Error::Mount);
        }
        signed::Local::mount_prepared(
            input,
            self.lease,
            online,
            cancelled,
            self.permit,
            dispatcher,
        )
        .map_err(Error::Mount)
    }
}
impl Control<'_> {
    fn art_prepare_response(&self, body: &[u8]) -> Result<zeroize::Zeroizing<Vec<u8>>, Error> {
        self.send_limited(
            "POST",
            "/v1/art/local/prepare",
            body,
            Duration::from_secs(10),
            MAX_RESPONSE,
        )
        .map_err(Error::Http)
    }
}
