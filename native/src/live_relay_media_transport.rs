//! Media TCP/TLS/WebSocket connections; background only, with addresses from trusted discovery/resolution owners.
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
    /// Addresses must match Prepared's fixed host; at most four candidates, with at most two seconds per connection.
    /// TLS always verifies the original hostname, never treating a resolved IP as the trusted identity.
    pub fn connect(prepared: Prepared, addresses: &[SocketAddr]) -> Result<Self, Error> {
        Self::connect_cancellable(prepared, addresses, &AtomicBool::new(false), |_| {})
    }

    /// Deliver interrupt immediately after TCP connection to cover TLS/upgrade; callbacks may only quickly register handles.
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

    /// Each step shares a 250ms total I/O budget that fragmented traffic cannot extend indefinitely; failure revokes the media worker.
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
