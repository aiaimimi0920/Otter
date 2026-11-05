//! 单Assets线程内的Wall流；每步有界IO，编码缓冲和解码帧均计入原共享像素池。
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
        // tungstenite的接收缓冲与返回payload可能并存；四个流及旧代际均共享512MiB池。
        let wire =
            crate::pixels::reserve(2 * packet::LIMIT + 64 * 1024).ok_or("Live编码预算不足")?;
        let prepared = lease
            .wall_media(client, key, session, format)
            .map_err(|_| "Live授权无效")?;
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
        .map_err(|_| "Live连接失败")?;
        if !upgraded.device || upgraded.protocol != packet::PROTOCOL {
            return Err("Live协议不匹配");
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
            return Err("Live已停止");
        }
        self.socket
            .get_mut()
            .base()
            .operation_deadline(Duration::from_millis(5))
            .map_err(|_| "Live期限失败")?;
        if let Some(ping) = self
            .liveness
            .poll(Instant::now())
            .map_err(|_| "Live时钟失败")?
        {
            self.socket
                .send(Message::Ping(ping.to_vec()))
                .map_err(|_| "Live已断开")?;
        }
        match self.socket.read() {
            Ok(Message::Binary(bytes)) => {
                let (identity, received) = self.queue.watermark();
                let frame = packet::decode(&bytes, self.format, identity, received, &stopped)?;
                if stopped() {
                    return Err("Live已停止");
                }
                self.queue.push(frame);
                self.last = Instant::now();
            }
            Ok(Message::Ping(bytes)) => {
                self.socket
                    .send(Message::Pong(bytes))
                    .map_err(|_| "Live已断开")?;
            }
            Ok(Message::Pong(bytes)) => {
                self.liveness.observe(&bytes, Instant::now());
            }
            Ok(Message::Close(_)) => return Err("Live来源已关闭或授权失效"),
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            _ => return Err("Live媒体无效或已断开"),
        }
        if self.last.elapsed() > Duration::from_secs(5) {
            return Err("Live来源已停滞");
        }
        Ok(())
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        // 不在析构无限等待远端close；共享interrupt owner随后释放，socket Drop关闭本端。
        let _ = self
            .socket
            .get_mut()
            .base()
            .operation_deadline(Duration::from_millis(5));
        let _ = self.socket.close(None);
    }
}
