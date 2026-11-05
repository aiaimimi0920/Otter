//! 媒体 TCP/TLS/WebSocket 连接；仅后台调用，解析地址由可信发现/解析 owner 提供。
use crate::{
    byte_budget::Budget,
    live_relay_media_endpoint::Prepared,
    live_relay_media_liveness::Liveness,
    live_relay_media_socket::{self, Authorization, Step},
    live_relay_media_worker::Worker,
    live_relay_receive::Profile,
    loom_bridge_stream::Interrupt,
    media_websocket::Stream,
};
use std::{
    io,
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};
use tungstenite::WebSocket;
#[path = "live_relay_source_socket.rs"]
mod source;
pub(crate) use source::{SourceStep, VideoPolicy, BUFFER_BUDGET};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Address,
    Connect,
    Deadline,
    Tls,
    Upgrade,
    Profile,
    Media(live_relay_media_socket::Error),
}
impl Error {
    pub(crate) fn recoverable(self) -> bool {
        matches!(self, Self::Connect | Self::Deadline)
            || matches!(self, Self::Media(error) if error.recoverable())
    }
}

pub struct Connection {
    socket: WebSocket<Stream>,
    profile: Profile,
    authorization: Authorization,
    liveness: Liveness,
}
impl Connection {
    /// 地址必须对应 Prepared 的固定 host；最多四个候选，每次连接最多两秒。
    /// TLS 始终按原始 hostname 验证，而不是把解析出的 IP 当信任身份。
    pub fn connect(prepared: Prepared, addresses: &[SocketAddr]) -> Result<Self, Error> {
        Self::connect_cancellable(prepared, addresses, &AtomicBool::new(false), |_| {})
    }

    /// TCP 建立后立即交付 interrupt，覆盖 TLS/upgrade 阶段；回调只能快速登记句柄。
    pub fn connect_cancellable(
        prepared: Prepared,
        addresses: &[SocketAddr],
        stop: &AtomicBool,
        connected: impl FnOnce(Interrupt),
    ) -> Result<Self, Error> {
        let upgraded = crate::media_websocket::connect(
            prepared,
            addresses,
            stop,
            connected,
            live_relay_media_socket::config(),
        )?;
        let profile = Profile::negotiated(Some(&upgraded.protocol)).map_err(|_| Error::Profile)?;
        Ok(Self {
            socket: upgraded.socket,
            profile,
            authorization: if upgraded.device {
                Authorization::Device
            } else {
                Authorization::Other
            },
            liveness: Liveness::new(Instant::now()),
        })
    }

    pub fn interrupt_handle(&mut self) -> Result<Interrupt, Error> {
        self.socket
            .get_mut()
            .base()
            .interrupt_handle()
            .map_err(|_| Error::Connect)
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn rtt(&self) -> Option<Duration> {
        self.liveness.rtt()
    }

    /// 每步共用 250ms 总 I/O 预算，不能由碎片流量无限延长；失败撤销媒体 worker。
    pub fn service(&mut self, budget: &Arc<Budget>, worker: &Worker) -> Result<Step, Error> {
        if self
            .socket
            .get_mut()
            .base()
            .operation_deadline(Duration::from_millis(250))
            .is_err()
        {
            worker.request_stop();
            return Err(Error::Deadline);
        }
        live_relay_media_socket::service(
            &mut self.socket,
            self.profile,
            self.authorization,
            budget,
            worker,
            &mut self.liveness,
        )
        .map_err(Error::Media)
    }

    pub(super) fn service_resumable(
        &mut self,
        budget: &Arc<Budget>,
        worker: &Worker,
        after: Option<crate::live_relay_viewer_frames::Cursor>,
    ) -> Result<(Step, Option<crate::live_relay_frame::Metadata>), Error> {
        self.socket
            .get_mut()
            .base()
            .operation_deadline(Duration::from_millis(250))
            .map_err(|_| Error::Deadline)?;
        live_relay_media_socket::service_resumable(
            &mut self.socket,
            self.profile,
            self.authorization,
            budget,
            worker,
            &mut self.liveness,
            after,
        )
        .map_err(Error::Media)
    }
}

#[cfg(test)]
#[path = "live_relay_media_transport_tests.rs"]
mod tests;
