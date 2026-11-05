//! 单条工作流请求交换；后台调用，调用方负责准入连接、复核 receipt 和消费成功回执。
use super::Request;
use crate::{byte_budget::Budget, loom_bridge_client::BridgeSocket};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tungstenite::Message;
const RESPONSE_LIMIT: usize = 1024 * 1024;

#[derive(Debug, PartialEq)]
pub enum Error {
    Cancelled,
    Budget,
    Send,
    Closed,
    Timeout,
    InvalidResponse,
    Rejected,
}

/// 不返回远端任意错误文本，避免凭据/路径通过日志或 UI 回显。
pub fn response(bytes: &[u8], request: &Request) -> Result<bool, Error> {
    if bytes.len() > RESPONSE_LIMIT {
        return Err(Error::InvalidResponse);
    }
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Ok(false);
    };
    if value["protocolVersion"] != "loom.hook.v1"
        || value["requestId"].as_str() != Some(request.id())
        || !value["method"].is_null()
    {
        return Ok(false);
    }
    match value["status"].as_str() {
        Some("succeeded")
            if value["data"]["workflowId"].as_str() == Some(request.workflow_id()) =>
        {
            Ok(true)
        }
        Some("failed" | "cancelled") => Err(Error::Rejected),
        _ => Err(Error::InvalidResponse),
    }
}

/// 一次尝试，五秒总期限；超时不自动重发，也不推进批次或修改本地来源。
pub fn run(
    socket: BridgeSocket,
    request: &Request,
    budget: &Arc<Budget>,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    check(cancel)?;
    // Tungstenite 接管 Text；其发送副本须独立准入，不能借用批次租约重复记账。
    let mut lease = budget.reserve(request.bytes().len()).ok_or(Error::Budget)?;
    let mut message = String::new();
    message
        .try_reserve_exact(request.bytes().len())
        .map_err(|_| Error::Budget)?;
    if !lease.grow_to(message.capacity()) {
        return Err(Error::Budget);
    }
    message.push_str(std::str::from_utf8(request.bytes()).map_err(|_| Error::InvalidResponse)?);
    // 内层先销毁 socket（含可能尚未发送的 Text），外层才归还租约。
    exchange(socket, request, message, cancel)
}
fn exchange(
    mut socket: BridgeSocket,
    request: &Request,
    message: String,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    socket.set_config(|config| {
        config.max_message_size = Some(RESPONSE_LIMIT);
        config.max_frame_size = Some(RESPONSE_LIMIT);
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    set_deadline(&mut socket, deadline, Duration::from_secs(5))?;
    check(cancel)?;
    socket
        .send(Message::Text(message))
        .map_err(|_| Error::Send)?;
    loop {
        check(cancel)?;
        set_deadline(&mut socket, deadline, Duration::from_millis(250))?;
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if matches!(
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
        check(cancel)?;
        match message {
            Message::Text(text) if response(text.as_bytes(), request)? => return Ok(()),
            Message::Ping(_) => {
                set_deadline(&mut socket, deadline, Duration::from_millis(250))?;
                socket.flush().map_err(|_| Error::Closed)?;
            }
            Message::Close(_) => return Err(Error::Closed),
            _ => {}
        }
    }
}
fn check(cancel: &AtomicBool) -> Result<(), Error> {
    if cancel.load(Ordering::Acquire) {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn set_deadline(socket: &mut BridgeSocket, deadline: Instant, cap: Duration) -> Result<(), Error> {
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
