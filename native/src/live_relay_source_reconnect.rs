//! 原Source线程内有限恢复；服务认证的实际帧水位才重置次数，不以握手/本地send自证接收。
use super::*;
use crate::live_relay_media_transport::Connection;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct Recovery {
    attempts: u32,
    active: bool,
}
impl Recovery {
    pub fn active(&self) -> bool {
        self.active
    }
    pub fn retry(
        &mut self,
        exit: Exit,
        client: &crate::device_session_client::Client,
        lease: &Lease,
        signal: &Signal,
    ) -> Result<(), Exit> {
        if !matches!(exit, Exit::Media(e) if e.recoverable())
            && !matches!(exit, Exit::Http(e) if e.retryable_read())
        {
            return Err(exit);
        }
        if self.attempts >= 4 {
            return Err(exit);
        }
        self.active = true;
        signal.recovery(true);
        let delay = Duration::from_millis((400u64 << self.attempts).min(2000));
        self.attempts += 1;
        wait(client, lease, signal, delay)
    }
    pub fn confirm(
        &mut self,
        client: &crate::device_session_client::Client,
        lease: &Lease,
        publication: &Publication,
        signal: &Signal,
        sent: u64,
    ) -> Result<(), Exit> {
        if !self.active {
            return Ok(());
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            check(client, lease, signal)?;
            if lease
                .control(client)
                .map_err(|_| Exit::Lease)?
                .source_progress(publication, sent)
                .map_err(Exit::Http)?
                .is_some()
            {
                self.attempts = 0;
                self.active = false;
                signal.recovery(false);
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Exit::Media(
                    crate::live_relay_media_transport::Error::Media(
                        crate::live_relay_media_socket::Error::Liveness,
                    ),
                ));
            }
            wait(client, lease, signal, Duration::from_millis(100))?;
        }
    }
}
pub(super) fn check(
    client: &crate::device_session_client::Client,
    lease: &Lease,
    signal: &Signal,
) -> Result<(), Exit> {
    if signal.stop.load(Ordering::Acquire) {
        return Err(Exit::Stopped);
    }
    lease
        .validate(client.origin(), now()?)
        .map_err(|_| Exit::Lease)
}
fn wait(
    client: &crate::device_session_client::Client,
    lease: &Lease,
    signal: &Signal,
    delay: Duration,
) -> Result<(), Exit> {
    let deadline = Instant::now() + delay;
    loop {
        check(client, lease, signal)?;
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        std::thread::park_timeout(remaining);
    }
}
pub(super) fn connect(
    client: &crate::device_session_client::Client,
    lease: &Lease,
    publication: &Publication,
    signal: &Signal,
    video: &mut video::Video,
) -> Result<Connection, Exit> {
    check(client, lease, signal)?;
    let mut prepared = lease
        .source_media(client, publication.session())
        .map_err(Exit::MediaStart)?;
    if publication.h264() && video.available() {
        prepared.source_h264();
    }
    let mut socket =
        Connection::connect_cancellable(prepared, &client.addresses(), &signal.stop, |s| {
            signal.install(s)
        })
        .map_err(Exit::Media)?;
    socket.source_limits().map_err(Exit::Media)?;
    if socket.profile() != crate::live_relay_receive::Profile::H264 {
        video.legacy();
    }
    Ok(socket)
}
