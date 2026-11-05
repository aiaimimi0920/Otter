//! 后台单次 Art 消息交换；不创建线程或队列，预览由调用者即时接管资源生命周期。
use crate::{
    graph_requests::Request,
    loom_art_response::{self, Event},
    loom_bridge_client::BridgeSocket,
    workspace_id::UnitId,
};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tungstenite::Message;

#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidTimeout,
    Send,
    Closed,
    Timeout,
    Response(loom_art_response::Error),
    Cancelled,
}

/// 输入资源 owner 必须活到本调用返回。on_update 返回 false 会结束交换并关闭本地连接。
/// 调用者必须使用完整请求令牌过滤 UI 更新，并负责 preview/result 内资源的释放协议。
pub fn run(
    socket: BridgeSocket,
    message: String,
    unit: &UnitId,
    request: &Request,
    timeout: Duration,
    on_update: impl FnMut(Event) -> bool,
) -> Result<Event, Error> {
    run_cancellable(socket, message, unit, request, timeout, None, on_update)
}
/// 取消任务以 250 ms 读预算复核标志，避免只依赖 Windows shutdown 唤醒竞态。
pub fn run_cancellable(
    mut socket: BridgeSocket,
    message: String,
    unit: &UnitId,
    request: &Request,
    timeout: Duration,
    cancel: Option<&AtomicBool>,
    mut on_update: impl FnMut(Event) -> bool,
) -> Result<Event, Error> {
    let deadline = Instant::now()
        .checked_add(timeout)
        .filter(|_| !timeout.is_zero())
        .ok_or(Error::InvalidTimeout)?;
    set_deadline(&mut socket, deadline, Some(Duration::from_secs(5)))?;
    socket
        .send(Message::Text(message))
        .map_err(|_| Error::Send)?;
    loop {
        if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
            return Err(Error::Cancelled);
        }
        set_deadline(
            &mut socket,
            deadline,
            cancel.map(|_| Duration::from_millis(250)),
        )?;
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if cancel.is_some()
                    && matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
            {
                continue
            }
            Err(_) => {
                return Err(if Instant::now() >= deadline {
                    Error::Timeout
                } else {
                    Error::Closed
                })
            }
        };
        match message {
            Message::Text(text) => match loom_art_response::parse(text.as_bytes(), unit, request)
                .map_err(Error::Response)?
            {
                Event::Ignore => {}
                event @ (Event::Result { .. } | Event::Failure(_)) => return Ok(event),
                update => {
                    if !on_update(update) {
                        return Err(Error::Cancelled);
                    }
                }
            },
            Message::Ping(_) => {
                // Tungstenite 已排队 Pong；单独有界 flush，不能因心跳刷新总响应期限。
                set_deadline(&mut socket, deadline, Some(Duration::from_secs(5)))?;
                socket.flush().map_err(|_| Error::Closed)?;
            }
            Message::Close(_) => return Err(Error::Closed),
            _ => {}
        }
    }
}

fn set_deadline(
    socket: &mut BridgeSocket,
    deadline: Instant,
    cap: Option<Duration>,
) -> Result<(), Error> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Error::Timeout);
    }
    socket
        .get_mut()
        .sock
        .operation_deadline(cap.map_or(remaining, |cap| remaining.min(cap)))
        .map_err(|_| Error::Closed)
}
