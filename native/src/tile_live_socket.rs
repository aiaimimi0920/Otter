//! Wall streams within one Assets thread; bounded I/O per step, with encoded buffers and decoded frames charged to the original shared pixel pool.
use crate::{
    device_authorization_lease::Lease,
    device_session_client::Client,
    live_relay_media_liveness::Liveness,
    media_websocket::{self, Stream},
    tile_live_packet::{self as packet, WallMediaFormat},
    tile_live_queue::Queue,
    tile_shared::Key,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket};
pub(crate) type Interrupts = Arc<Mutex<Vec<crate::loom_bridge_stream::Interrupt>>>;
pub(crate) struct Receiver {
    socket: WebSocket<Stream>,
    pub queue: Queue,
    format: WallMediaFormat,
    liveness: Liveness,
    last: Instant,
    _wire: crate::byte_budget::Lease,
}
impl Receiver {
    pub fn connect(
        client: &Client,
        lease: &Lease,
        key: &Key,
        session: &str,
        format: WallMediaFormat,
        interrupts: &Interrupts,
    ) -> Result<Self, &'static str> {
        // tungstenite receive buffers and returned payloads may coexist; all four streams and old generations share the 512MiB pool.
        let wire = crate::pixels::reserve(2 * packet::LIMIT + 64 * 1024)
            .ok_or("tile-live-encode-budget")?;
        let prepared = lease
            .wall_media(client, key, session, format)
            .map_err(|_| "tile-live-authorization-invalid")?;
        let mut config = crate::live_relay_media_socket::config();
        config.max_message_size = Some(packet::LIMIT);
        config.max_frame_size = Some(packet::LIMIT);
        let upgraded = media_websocket::connect(
            prepared,
            &client.addresses(),
            lease.preparation_cancel(),
            |i| {
                let mut handles = interrupts.lock().unwrap_or_else(|e| e.into_inner());
                if lease.is_revoked() {
                    let _ = i.cancel();
                } else {
                    handles.push(i);
                }
            },
            config,
        )
        .map_err(|_| "tile-live-connection-failed")?;
        if !upgraded.device || upgraded.protocol != packet::PROTOCOL {
            return Err("tile-live-protocol-mismatch");
        }
        Ok(Self {
            socket: upgraded.socket,
            queue: Queue::default(),
            format,
            liveness: Liveness::new(Instant::now()),
            last: Instant::now(),
            _wire: wire,
        })
    }
    pub fn poll(&mut self, stopped: impl Fn() -> bool) -> Result<(), &'static str> {
        if stopped() {
            return Err("tile-live-stopped");
        }
        self.socket
            .get_mut()
            .base()
            .operation_deadline(Duration::from_millis(5))
            .map_err(|_| "tile-live-deadline-failed")?;
        if let Some(ping) = self
            .liveness
            .poll(Instant::now())
            .map_err(|_| "tile-live-clock-failed")?
        {
            self.socket
                .send(Message::Ping(ping.to_vec()))
                .map_err(|_| "tile-live-disconnected")?;
        }
        match self.socket.read() {
            Ok(Message::Binary(bytes)) => {
                let (identity, received) = self.queue.watermark();
                let frame = packet::decode(&bytes, self.format, identity, received, &stopped)?;
                if stopped() {
                    return Err("tile-live-stopped");
                }
                self.queue.push(frame);
                self.last = Instant::now();
            }
            Ok(Message::Ping(bytes)) => {
                self.socket
                    .send(Message::Pong(bytes))
                    .map_err(|_| "tile-live-disconnected")?;
            }
            Ok(Message::Pong(bytes)) => {
                self.liveness.observe(&bytes, Instant::now());
            }
            Ok(Message::Close(_)) => return Err("tile-live-source-closed"),
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            _ => return Err("tile-live-media-invalid"),
        }
        if self.last.elapsed() > Duration::from_secs(5) {
            return Err("tile-live-source-stalled");
        }
        Ok(())
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        // Never wait indefinitely for remote close during destruction; release the shared interrupt owner afterward, and socket Drop closes the local endpoint.
        let _ = self
            .socket
            .get_mut()
            .base()
            .operation_deadline(Duration::from_millis(5));
        let _ = self.socket.close(None);
    }
}
