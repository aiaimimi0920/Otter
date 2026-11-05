//! 已准入 WebSocket 的单步媒体服务；连接/认证/TLS/重连由 transport owner 负责。
use crate::{
    byte_budget::Budget,
    live_relay_frame::{HEADER_LEN, MAX_FRAME_BYTES},
    live_relay_media_liveness::Liveness,
    live_relay_media_pipeline::Progress,
    live_relay_media_worker::Worker,
    live_relay_receive::{Frame, Profile},
};
use std::{
    io::{Read, Write},
    sync::Arc,
    time::Instant,
};
use tungstenite::{
    protocol::{frame::coding::CloseCode, WebSocketConfig},
    Message, WebSocket,
};
const LIMIT: usize = MAX_FRAME_BYTES + HEADER_LEN;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Authorization {
    Device,
    Other,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Idle,
    Queued,
    Closed,
    DeviceRevoked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stopped,
    Limits,
    Transport,
    Message,
    Frame,
    Worker,
    Liveness,
}
impl Error {
    pub(super) fn recoverable(self) -> bool {
        matches!(self, Self::Transport | Self::Liveness)
    }
    pub(crate) fn from_socket(error: tungstenite::Error) -> Self {
        match error {
            tungstenite::Error::Io(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::UnexpectedEof
                        | std::io::ErrorKind::NotConnected
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                ) =>
            {
                Self::Transport
            }
            tungstenite::Error::ConnectionClosed
            | tungstenite::Error::AlreadyClosed
            | tungstenite::Error::Protocol(
                tungstenite::error::ProtocolError::ResetWithoutClosingHandshake,
            ) => Self::Transport,
            _ => Self::Message,
        }
    }
}
pub fn config() -> WebSocketConfig {
    WebSocketConfig {
        write_buffer_size: 0,
        max_write_buffer_size: 64 * 1024,
        max_message_size: Some(LIMIT),
        max_frame_size: Some(LIMIT),
        ..Default::default()
    }
}
/// transport 必须先完成可信端点/TLS/凭据/实际子协议验证，并配置有限 read/write timeout。
/// 非阻塞/超时读取返回 Idle；本方法不生成凭据，也不从 payload 推断协商 profile。
pub fn service<S: Read + Write>(
    socket: &mut WebSocket<S>,
    profile: Profile,
    authorization: Authorization,
    budget: &Arc<Budget>,
    worker: &Worker,
    liveness: &mut Liveness,
) -> Result<Step, Error> {
    let result = service_inner(
        socket,
        profile,
        authorization,
        budget,
        worker,
        liveness,
        None,
    )
    .map(|(step, _)| step);
    if result.is_err() {
        worker.request_stop();
    }
    result
}
/// 只由同一授权/epoch的恢复owner使用；传输损失保留原worker，协议/预算错误仍失败关闭。
pub(super) fn service_resumable<S: Read + Write>(
    socket: &mut WebSocket<S>,
    profile: Profile,
    authorization: Authorization,
    budget: &Arc<Budget>,
    worker: &Worker,
    liveness: &mut Liveness,
    after: Option<crate::live_relay_viewer_frames::Cursor>,
) -> Result<(Step, Option<crate::live_relay_frame::Metadata>), Error> {
    let result = service_inner(
        socket,
        profile,
        authorization,
        budget,
        worker,
        liveness,
        after,
    );
    if result.as_ref().is_err_and(|error| !error.recoverable()) {
        worker.request_stop();
    }
    result
}
fn service_inner<S: Read + Write>(
    socket: &mut WebSocket<S>,
    profile: Profile,
    authorization: Authorization,
    budget: &Arc<Budget>,
    worker: &Worker,
    liveness: &mut Liveness,
    after: Option<crate::live_relay_viewer_frames::Cursor>,
) -> Result<(Step, Option<crate::live_relay_frame::Metadata>), Error> {
    if !worker.is_current() {
        return Err(Error::Stopped);
    }
    let limits = socket.get_config();
    if limits.max_message_size.is_none_or(|v| v > LIMIT)
        || limits.max_frame_size.is_none_or(|v| v > LIMIT)
    {
        return Err(Error::Limits);
    }
    if let Some(payload) = liveness.poll(Instant::now()).map_err(|_| Error::Liveness)? {
        socket
            .send(Message::Ping(payload.to_vec()))
            .map_err(|_| Error::Transport)?;
    }
    for _ in 0..4 {
        let Some(reply) = worker.try_reply() else {
            break;
        };
        let (kind, epoch) = match reply {
            Progress::RequestKeyframe { epoch } => ("keyframe_request", epoch),
            Progress::Fallback { epoch, .. } => ("video_fallback", epoch),
            Progress::Pending | Progress::Published(_) => continue,
        };
        if !worker.is_current() {
            return Err(Error::Stopped);
        }
        socket
            .send(Message::Text(
                serde_json::json!({"type":kind,"epoch":epoch}).to_string(),
            ))
            .map_err(|_| Error::Transport)?;
    }
    match socket.read() {
        Ok(Message::Binary(bytes)) => {
            let frame = Frame::from_message(bytes, profile, budget).map_err(|_| Error::Frame)?;
            let metadata = frame.metadata();
            // H264重连需要旧IDR预热参考链；raw/JPEG重复帧不能重新发布或误作新进度。
            if metadata.codec != crate::live_relay_frame::Codec::H264
                && after
                    .is_some_and(|c| c.epoch == metadata.epoch && metadata.frame_id <= c.frame_id)
            {
                return Ok((Step::Idle, Some(metadata)));
            }
            worker.submit(frame).map_err(|_| Error::Worker)?;
            Ok((Step::Queued, Some(metadata)))
        }
        Ok(Message::Ping(bytes)) => {
            socket
                .send(Message::Pong(bytes))
                .map_err(|_| Error::Transport)?;
            Ok((Step::Idle, None))
        }
        Ok(Message::Pong(bytes)) => {
            liveness.observe(&bytes, Instant::now());
            Ok((Step::Idle, None))
        }
        Ok(Message::Close(close)) => {
            let revoked = authorization == Authorization::Device
                && close.is_some_and(|c| {
                    c.code == CloseCode::Policy && c.reason == "live_media_device_revoked"
                });
            if revoked {
                worker.revoke();
                Ok((Step::DeviceRevoked, None))
            } else {
                worker.request_stop();
                Ok((Step::Closed, None))
            }
        }
        Err(tungstenite::Error::Io(e))
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            Ok((Step::Idle, None))
        }
        // 无Close的EOF可恢复；其余协议/UTF-8错误不是临时网络损失。
        Err(error) => Err(Error::from_socket(error)),
        Ok(_) => Err(Error::Message),
    }
}
#[cfg(test)]
#[path = "live_relay_media_socket_tests.rs"]
mod tests;
