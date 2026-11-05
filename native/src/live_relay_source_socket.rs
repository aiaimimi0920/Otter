//! Source的raw/H264分片复用原TLS/socket；真实video_policy只在协商的H264连接接收。
use super::*;
#[path = "live_relay_source_policy.rs"]
mod policy;
use crate::live_relay_frame::Metadata;
pub(crate) use policy::VideoPolicy;
use tungstenite::{
    protocol::frame::{
        coding::{Data, OpCode},
        Frame,
    },
    Message,
};
const CHUNK: usize = 16 * 1024;
pub(crate) const BUFFER_BUDGET: usize = 512 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceStep {
    Idle,
    Closed,
    Revoked,
    Policy(VideoPolicy),
}
impl Connection {
    pub(crate) fn source_limits(&mut self) -> Result<(), Error> {
        if !matches!(self.profile, Profile::Legacy | Profile::H264)
            || self.authorization != Authorization::Device
        {
            return Err(Error::Profile);
        }
        self.socket.set_config(|c| {
            c.max_message_size = Some(64 * 1024);
            c.max_frame_size = Some(64 * 1024);
        });
        Ok(())
    }
    pub(crate) fn send_source(
        &mut self,
        metadata: Metadata,
        pixels: &[u8],
        check: impl Fn() -> bool,
    ) -> Result<(), Error> {
        if metadata.codec == crate::live_relay_frame::Codec::H264 && self.profile != Profile::H264 {
            return Err(Error::Profile);
        }
        let header = metadata
            .encode_header(pixels.len())
            .map_err(|_| Error::Profile)?;
        self.socket
            .get_mut()
            .base()
            .operation_deadline(Duration::from_millis(250))
            .map_err(|_| Error::Deadline)?;
        if !check() {
            return Err(Error::Stopped);
        }
        self.socket
            .send(Message::Frame(Frame::message(
                header.to_vec(),
                OpCode::Data(Data::Binary),
                false,
            )))
            .map_err(source_error)?;
        let count = pixels.len().div_ceil(CHUNK);
        for (index, chunk) in pixels.chunks(CHUNK).enumerate() {
            if !check() {
                return Err(Error::Stopped);
            }
            self.socket
                .send(Message::Frame(Frame::message(
                    chunk.to_vec(),
                    OpCode::Data(Data::Continue),
                    index + 1 == count,
                )))
                .map_err(source_error)?;
        }
        Ok(())
    }
    pub(crate) fn poll_source(&mut self) -> Result<SourceStep, Error> {
        use crate::live_relay_media_socket::Error as SocketError;
        self.socket
            .get_mut()
            .base()
            .operation_deadline(Duration::from_millis(5))
            .map_err(|_| Error::Deadline)?;
        if let Some(bytes) = self
            .liveness
            .poll(Instant::now())
            .map_err(|_| Error::Media(SocketError::Liveness))?
        {
            self.socket
                .send(Message::Ping(bytes.to_vec()))
                .map_err(source_error)?;
        }
        match self.socket.read() {
            Ok(Message::Text(text)) if self.profile == Profile::H264 => {
                return VideoPolicy::parse(&text)
                    .map(SourceStep::Policy)
                    .ok_or(Error::Media(SocketError::Message));
            }
            Ok(Message::Ping(bytes)) => {
                self.socket
                    .send(Message::Pong(bytes))
                    .map_err(source_error)?;
            }
            Ok(Message::Pong(bytes)) => {
                self.liveness.observe(&bytes, Instant::now());
            }
            Ok(Message::Close(close)) => {
                return Ok(
                    if close.is_some_and(|c| {
                        c.code == tungstenite::protocol::frame::coding::CloseCode::Policy
                            && c.reason == "live_media_device_revoked"
                    }) {
                        SourceStep::Revoked
                    } else {
                        SourceStep::Closed
                    },
                )
            }
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(source_error(error)),
            Ok(_) => return Err(Error::Media(SocketError::Message)),
        }
        Ok(SourceStep::Idle)
    }
}
fn source_error(error: tungstenite::Error) -> Error {
    Error::Media(live_relay_media_socket::Error::from_socket(error))
}
