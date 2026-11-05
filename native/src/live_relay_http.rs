//! 已授权 Live 控制请求；复用固定 origin HTTPS，协议结果不直接激活媒体发布许可。
use crate::{
    device_request_cancel::Cancel,
    device_session_attempt::Session,
    device_session_client::{Client, Request},
    live_relay_viewer_identity::{
        self as protocol, ControlBatch, ControlCursor, Identity, Snapshot,
    },
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::rand::{SecureRandom, SystemRandom};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use zeroize::Zeroizing;
#[path = "live_relay_join.rs"]
mod join;
pub use join::{Join, Prepared, RequesterCursor};
#[path = "live_relay_resume.rs"]
mod resume;
pub use resume::Resumed;
#[path = "live_relay_catalog.rs"]
mod catalog;
pub use catalog::CatalogEntry;
#[path = "surface_binding_http.rs"]
mod surface;
pub use surface::MountedSurface;
pub(crate) use surface::{declarative_scene, scene_for_runtime};
#[path = "surface_attach_http.rs"]
mod surface_attach;
#[path = "surface_event_http.rs"]
mod surface_event;
#[path = "surface_event_batch_http.rs"]
mod surface_event_batch;
pub(crate) use surface_event_batch::validate_batch;
pub(crate) use surface_event_batch::validate_batch_identity;
pub use surface_event_batch::{BatchError, BatchStop, BatchSubmission};
#[path = "surface_lifecycle_http.rs"]
mod surface_lifecycle;
#[path = "surface_recovery_http.rs"]
mod surface_recovery;
pub use surface_attach::Persistence;
pub use surface_event::SubmitError;
#[path = "surface_stream_http.rs"]
mod surface_stream;
pub use surface_recovery::RecoveryCandidate;
pub use surface_stream::StreamError;
#[path = "surface_convergence_http.rs"]
mod surface_convergence;
pub use surface_convergence::{ConvergenceError, ConvergenceOutcome};
#[path = "surface_confirmation_http.rs"]
mod surface_confirmation;
pub use surface_confirmation::{ConfirmationError, ConfirmationOutcome};
#[path = "surface_confirmation_decision_http.rs"]
mod surface_confirmation_decision;
pub use surface_confirmation_decision::DecisionSubmitError;
#[path = "surface_resource_http.rs"]
mod surface_resource;
pub(crate) use surface_resource::javascript::Entry as JavaScriptEntry;
pub use surface_resource::{ResourceError, ResourceLease, VerifiedResource};
#[path = "art_describe_http.rs"]
pub mod art_describe;
#[path = "art_prepare_http.rs"]
pub mod art_prepare;
#[path = "live_relay_viewer_input_http.rs"]
pub(crate) mod input;
#[path = "projection_delivery_http.rs"]
mod projection_delivery;
#[path = "projection_edit_http.rs"]
mod projection_edit;
#[path = "live_relay_source_http.rs"]
pub mod source;
#[path = "live_relay_viewer_trigger_http.rs"]
mod trigger;
#[path = "live_relay_wall_http.rs"]
mod wall;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Input,
    Discovery,
    Cursor,
    Credential,
    Clock,
    Random,
    AuthorizationRequired,
    Status(u16),
    Http(crate::device_session_client::Error),
    Protocol(protocol::Error),
    Business(crate::live_relay_viewer_business::Error),
    SurfaceBinding(crate::surface_live_binding::Error),
    SurfaceLifecycle(crate::surface_lifecycle_owner::Error),
    Projection(crate::projection_delivery::Error),
    ProjectionStopped(crate::projection_delivery::binding::StopReason),
    Sender(crate::projection_delivery::sender::Error),
}
impl Error {
    /// 只供幂等GET owner决定重试；同类POST错误仍可能已提交，绝不能沿此方法重放。
    pub(crate) fn retryable_read(self) -> bool {
        matches!(
            self,
            Self::Http(
                crate::device_session_client::Error::Connect
                    | crate::device_session_client::Error::Transport
                    | crate::device_session_client::Error::Deadline
            ) | Self::Status(408 | 429 | 500 | 502 | 503 | 504)
        )
    }
}
pub struct Attach<'a> {
    pub surface: &'a str,
    pub attachment: &'a str,
    pub sequence: u64,
    pub require_existing_membership: bool,
}
/// 借用已签发会话，不接受 UI 任意字符串充当 token；每次交换前后复核来源和到期时间。
pub struct Control<'a> {
    client: &'a Client,
    session: &'a Session,
    cancel: &'a Cancel,
}
impl<'a> Control<'a> {
    pub fn new(client: &'a Client, session: &'a Session, cancel: &'a Cancel) -> Self {
        Self {
            client,
            session,
            cancel,
        }
    }
    pub fn snapshot(&self, session: &str) -> Result<Snapshot, Error> {
        let bytes = self.send("GET", &path(session, None)?, &[], Duration::from_secs(10))?;
        Snapshot::parse(&bytes, session).map_err(Error::Protocol)
    }
    /// 错误后的关闭确认必须再次取得认证 member snapshot；404 本身绝不构成关闭事实。
    pub fn terminal(&self, expected: &Identity) -> Result<bool, Error> {
        let bytes = self.send(
            "GET",
            &path(expected.session(), None)?,
            &[],
            Duration::from_secs(10),
        )?;
        protocol::terminal_snapshot(&bytes, expected, self.session.device_id())
            .map_err(Error::Protocol)
    }
    pub fn attach(&self, expected: &Identity, options: Attach<'_>) -> Result<Snapshot, Error> {
        self.attach_business(expected, options)
            .map(|(snapshot, _)| snapshot)
    }
    fn attach_business(
        &self,
        expected: &Identity,
        options: Attach<'_>,
    ) -> Result<(Snapshot, crate::live_relay_viewer_business::Business), Error> {
        if !protocol::identifier(options.surface)
            || !protocol::identifier(options.attachment)
            || options.sequence == 0
        {
            return Err(Error::Input);
        }
        let mut body = serde_json::json!({
            "surfaceInstanceId":options.surface, "attachmentId":options.attachment,
            "envelope":self.viewer_ack(expected, options.sequence)?
        });
        if options.require_existing_membership {
            body["requireExistingMembership"] = true.into();
        }
        let body = serde_json::to_vec(&body).map_err(|_| Error::Input)?;
        let bytes = self.send(
            "POST",
            &path(expected.session(), Some("viewers"))?,
            &body,
            Duration::from_secs(15),
        )?;
        self.attached_viewer(expected, &bytes)
    }
    fn viewer_ack(&self, expected: &Identity, sequence: u64) -> Result<serde_json::Value, Error> {
        if sequence == 0 {
            return Err(Error::Input);
        }
        Ok(
            serde_json::json!({"protocolVersion":protocol::PROTOCOL,"sessionId":expected.session(),
            "epoch":expected.epoch(),"sequence":sequence,"messageType":"session_ack",
            "payload":{"accepted":true,"responderDeviceId":self.session.device_id()}}),
        )
    }
    fn attached_viewer(
        &self,
        expected: &Identity,
        bytes: &[u8],
    ) -> Result<(Snapshot, crate::live_relay_viewer_business::Business), Error> {
        let snapshot = Snapshot::parse(bytes, expected.session()).map_err(Error::Protocol)?;
        snapshot
            .renewal(expected, self.session.device_id())
            .map_err(Error::Protocol)?;
        let business = crate::live_relay_viewer_business::Business::from_snapshot(bytes)
            .map_err(Error::Business)?;
        Ok((snapshot, business))
    }
    pub fn events(&self, session: &str, cursor: &ControlCursor) -> Result<ControlBatch, Error> {
        self.events_wait(session, cursor, 750)
    }
    pub(crate) fn events_wait(
        &self,
        session: &str,
        cursor: &ControlCursor,
        wait: u16,
    ) -> Result<ControlBatch, Error> {
        if wait > 750 {
            return Err(Error::Input);
        }
        let bytes = self.history(session, cursor.after(), wait)?;
        cursor.prepare(&bytes).map_err(Error::Protocol)
    }
    /// 首次 bootstrap 必须匹配本次加入事件，不能直接采用服务端 next 跳过撤权。
    pub fn bootstrap_cursor(&self, attached: &Snapshot) -> Result<u64, Error> {
        let bytes = self.history(attached.identity().session(), 0, 0)?;
        protocol::join_cursor(attached, self.session.device_id(), &bytes).map_err(Error::Protocol)
    }
    fn history(
        &self,
        session: &str,
        after: u64,
        timeout: u16,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        self.send(
            "GET",
            &format!(
                "{}?after={after}&timeoutMs={timeout}",
                path(session, Some("events"))?
            ),
            &[],
            Duration::from_secs(3),
        )
    }
    fn send(
        &self,
        method: &'static str,
        path: &str,
        body: &[u8],
        timeout: Duration,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        self.send_limited(method, path, body, timeout, protocol::MAX_JSON)
    }
    fn send_limited(
        &self,
        method: &'static str,
        path: &str,
        body: &[u8],
        timeout: Duration,
        limit: usize,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        self.send_until(method, path, body, Instant::now() + timeout, limit)
    }
    fn send_until(
        &self,
        method: &'static str,
        path: &str,
        body: &[u8],
        deadline: Instant,
        limit: usize,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        self.send_resource_until(method, path, body, deadline, limit, None)
    }
    fn send_resource_until(
        &self,
        method: &'static str,
        path: &str,
        body: &[u8],
        deadline: Instant,
        limit: usize,
        resource: Option<&ResourceLease>,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        let (status, bytes) = self.exchange_until(method, path, body, deadline, limit, resource)?;
        if status == 401 {
            return Err(Error::AuthorizationRequired);
        }
        if !(200..300).contains(&status) {
            return Err(Error::Status(status));
        }
        self.session
            .token_for(self.client.origin(), now()?)
            .map_err(|_| Error::Credential)?;
        Ok(bytes)
    }
    fn exchange_until(
        &self,
        method: &'static str,
        path: &str,
        body: &[u8],
        deadline: Instant,
        limit: usize,
        resource: Option<&ResourceLease>,
    ) -> Result<(u16, Zeroizing<Vec<u8>>), Error> {
        let token = self
            .session
            .token_for(self.client.origin(), now()?)
            .map_err(|_| Error::Credential)?;
        let mut nonce = [0u8; 24];
        SystemRandom::new()
            .fill(&mut nonce)
            .map_err(|_| Error::Random)?;
        let nonce = URL_SAFE_NO_PAD.encode(nonce);
        let result = self.client.exchange_resource(
            Request {
                method,
                path,
                body,
                device: Some((token, &nonce)),
                limit,
            },
            resource,
            self.cancel.flag(),
            &mut |handle| self.cancel.install(handle),
            deadline,
        );
        self.cancel.clear();
        result.map_err(Error::Http)
    }
}
fn now() -> Result<u64, Error> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Clock)?;
    u64::try_from(duration.as_millis()).map_err(|_| Error::Clock)
}
fn path(session: &str, suffix: Option<&str>) -> Result<String, Error> {
    if !protocol::identifier(session) || matches!(session, "." | "..") {
        return Err(Error::Input);
    }
    // 单独转义 session segment，允许合同中的 ':'/'/' 但不能改写资源路径。
    let mut url = url::Url::parse("https://path.invalid/").map_err(|_| Error::Input)?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| Error::Input)?;
        segments.clear().extend(["v1", "live", "sessions", session]);
        if let Some(suffix) = suffix {
            segments.push(suffix);
        }
    }
    Ok(url.path().to_owned())
}
