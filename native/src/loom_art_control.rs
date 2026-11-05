//! 取消/资源释放控制通道，单次响应最多 1 MiB；不创建线程，也不记录远端原文。
use crate::{graph_requests::Request, loom_bridge_client::BridgeSocket, workspace_id::UnitId};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tungstenite::Message;
const MAX_BYTES: usize = 1024 * 1024;
#[derive(Debug, PartialEq)]
pub enum Error {
    Encoding,
    Connect(crate::loom_bridge_client::Error),
    InvalidHandles,
    InvalidResponse,
    Send,
    Closed,
    Timeout,
    TooLarge,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Succeeded,
    RequestNotFound,
    Failed,
    Cancelled,
}
/// 独立控制连接，固定执行身份。request_not_found 由调用方解释为已无活动执行。
pub fn cancel_authenticated(
    identity: &crate::loom_bridge_identity::Identity,
    unit: &UnitId,
    request: &Request,
) -> Result<Outcome, Error> {
    let message = crate::loom_art_wire::cancel(unit, request).map_err(|_| Error::Encoding)?;
    let socket = crate::loom_bridge_client::connect(identity, Duration::from_secs(5))
        .map_err(Error::Connect)?;
    run(socket, message, &request.id)
}

pub fn release_message(
    unit: &UnitId,
    execution: &Request,
    release_id: &UnitId,
    handles: &[&str],
) -> Result<String, Error> {
    if handles.is_empty()
        || handles.len() > 128
        || handles.iter().enumerate().any(|(index, handle)| {
            !handle.starts_with("Loom_Buffer_")
                || handle.len() <= 12
                || handle.len() > 256
                || handle.chars().any(char::is_control)
                || handles[..index].contains(handle)
        })
    {
        return Err(Error::InvalidHandles);
    }
    Ok(json!({"method":"loom.hook.art.resources.release","params":{
        "protocolVersion":"loom.hook.v1","requestId":release_id.as_str(),
        "executionRequestId":execution.id.as_str(),"nodeId":unit.as_str(),
        "generation":execution.generation,"deviceId":"device:local","handles":handles
    }})
    .to_string())
}

pub fn response(bytes: &[u8], request_id: &UnitId) -> Result<Option<Outcome>, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return Ok(None);
    };
    if value["protocolVersion"].as_str() != Some("loom.hook.v1")
        || value["requestId"].as_str() != Some(request_id.as_str())
        || !value["method"].is_null()
    {
        return Ok(None);
    }
    let outcome = match value["status"].as_str() {
        None => return Ok(None),
        Some("succeeded") => Outcome::Succeeded,
        Some("cancelled") => Outcome::Cancelled,
        Some("failed") if value["error"]["code"].as_str() == Some("request_not_found") => {
            Outcome::RequestNotFound
        }
        Some("failed") => Outcome::Failed,
        _ => return Err(Error::InvalidResponse),
    };
    Ok(Some(outcome))
}

/// 外层负责发现和认证；五秒总预算不因无关消息或心跳刷新。丢弃 socket 释放本地资源。
pub fn run(
    mut socket: BridgeSocket,
    message: String,
    request_id: &UnitId,
) -> Result<Outcome, Error> {
    if message.len() > MAX_BYTES {
        return Err(Error::TooLarge);
    }
    socket.set_config(|config| {
        config.max_message_size = Some(MAX_BYTES);
        config.max_frame_size = Some(MAX_BYTES);
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    reset_deadline(&mut socket, deadline)?;
    socket
        .send(Message::Text(message))
        .map_err(|_| Error::Send)?;
    loop {
        reset_deadline(&mut socket, deadline)?;
        match socket.read().map_err(|_| {
            if Instant::now() >= deadline {
                Error::Timeout
            } else {
                Error::Closed
            }
        })? {
            Message::Text(text) => {
                if let Some(outcome) = response(text.as_bytes(), request_id)? {
                    return Ok(outcome);
                }
            }
            Message::Ping(_) => {
                reset_deadline(&mut socket, deadline)?;
                socket.flush().map_err(|_| Error::Closed)?;
            }
            Message::Close(_) => return Err(Error::Closed),
            _ => {}
        }
    }
}
fn reset_deadline(socket: &mut BridgeSocket, deadline: Instant) -> Result<(), Error> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(Error::Timeout);
    }
    socket
        .get_mut()
        .sock
        .operation_deadline(remaining)
        .map_err(|_| Error::Closed)
}

#[cfg(test)]
#[path = "loom_art_control_tests.rs"]
mod tests;
