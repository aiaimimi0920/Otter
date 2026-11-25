//! Background-only capability handshake with fixed discovery-identity authentication; notices/heartbeats never extend total budget.
use crate::{
    loom_bridge_client::{self, BridgeSocket},
    loom_bridge_identity::Identity,
    loom_handshake::{self, Handshake},
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tungstenite::Message;
#[derive(Debug)]
pub enum Error {
    Cancelled,
    Timeout,
    Closed,
    TooLarge,
    Connect(loom_bridge_client::Error),
    Protocol(loom_handshake::Error),
}
pub fn fetch(identity: &Identity, cancel: &AtomicBool) -> Result<Handshake, Error> {
    open(identity, cancel).map(|(handshake, _)| handshake)
}
/// Continue extension negotiation on the same authenticated WebSocket; another connection's hook session is not a credential.
pub fn open(identity: &Identity, cancel: &AtomicBool) -> Result<(Handshake, BridgeSocket), Error> {
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    let message = loom_handshake::request("otter-native", env!("CARGO_PKG_VERSION"))
        .map_err(Error::Protocol)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut socket =
        loom_bridge_client::connect(identity, Duration::from_secs(10)).map_err(Error::Connect)?;
    if cancel.load(Ordering::Acquire) {
        return Err(Error::Cancelled);
    }
    socket.set_config(|config| {
        config.max_message_size = Some(loom_handshake::MAX_BYTES);
        config.max_frame_size = Some(loom_handshake::MAX_BYTES);
    });
    timeout(&mut socket, deadline, Duration::from_secs(3))?;
    socket
        .send(Message::Text(message))
        .map_err(|_| Error::Closed)?;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::Cancelled);
        }
        timeout(&mut socket, deadline, Duration::from_millis(250))?;
        match socket.read() {
            Ok(Message::Text(text)) => {
                if let Some(result) =
                    loom_handshake::response(text.as_bytes()).map_err(Error::Protocol)?
                {
                    if cancel.load(Ordering::Acquire) {
                        return Err(Error::Cancelled);
                    }
                    if Instant::now() >= deadline {
                        return Err(Error::Timeout);
                    }
                    return Ok((result, socket));
                }
            }
            Ok(Message::Ping(_)) => {
                timeout(&mut socket, deadline, Duration::from_secs(1))?;
                socket.flush().map_err(|_| Error::Closed)?;
            }
            Ok(Message::Close(_)) => return Err(Error::Closed),
            Ok(_) => {}
            Err(tungstenite::Error::Io(error))
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) => {}
            Err(tungstenite::Error::Capacity(_)) => return Err(Error::TooLarge),
            Err(_) => return Err(Error::Closed),
        }
    }
}
fn timeout(socket: &mut BridgeSocket, deadline: Instant, cap: Duration) -> Result<(), Error> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Error::Timeout);
    }
    socket
        .get_mut()
        .sock
        .operation_deadline(remaining.min(cap))
        .map_err(|_| Error::Closed)
}
