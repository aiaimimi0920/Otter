//! Bounded recovery in the original Source thread; reset counts only for service-authenticated actual frame watermarks, not handshake/local-send claims of receipt.
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
        wait_until(client, lease, signal, Instant::now() + delay)
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
            if signal.restart_media.load(Ordering::Acquire) {
                return Err(media_restart());
            }
            if Instant::now() >= deadline {
                break;
            }
            let progress = lease
                .execution_control(client)
                .map_err(|_| Exit::Lease)?
                .source_progress(publication, sent, deadline);
            // Do not use post-stop watermarks, but preserve received terminal authorization state.
            let progress = progress_after_check(progress, || {
                check(client, lease, signal)?;
                if signal.restart_media.load(Ordering::Acquire) {
                    return Err(media_restart());
                }
                Ok(())
            })?;
            let observed_at = Instant::now();
            if observed_at >= deadline {
                break;
            }
            if progress.is_some() {
                if !signal.finish_media_recovery() {
                    return Err(media_restart());
                }
                self.attempts = 0;
                self.active = false;
                return Ok(());
            }
            // Waiting and HTTP consume the same attempt budget; slow responses do not restart its clock.
            wait_until(
                client,
                lease,
                signal,
                (observed_at + Duration::from_millis(100)).min(deadline),
            )?;
        }
        Err(media_restart())
    }
}
pub(super) fn media_restart() -> Exit {
    Exit::Media(crate::live_relay_media_transport::Error::Media(
        crate::live_relay_media_socket::Error::Liveness,
    ))
}
pub(super) fn progress_after_check(
    progress: Result<Option<u64>, live_relay_http::Error>,
    check: impl FnOnce() -> Result<(), Exit>,
) -> Result<Option<u64>, Exit> {
    if let Err(error @ live_relay_http::Error::AuthorizationRequired) = progress {
        return Err(Exit::Http(error));
    }
    check()?;
    progress.map_err(Exit::Http)
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
fn wait_until(
    client: &crate::device_session_client::Client,
    lease: &Lease,
    signal: &Signal,
    deadline: Instant,
) -> Result<(), Exit> {
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
