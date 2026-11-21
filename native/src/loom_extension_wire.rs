//! 扩展握手/授权响应与目录推送；固定总截止时间，只有匹配 requestId 的响应可以完成操作。
use super::*;
use serde::Deserialize;
use serde_json::Value;
use tungstenite::Message;
pub(super) const MAX_BYTES: usize = 64 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    protocol: String,
    api_version: String,
    request_id: String,
    status: String,
    #[serde(default)]
    data: Value,
    #[serde(default, deserialize_with = "remote_error")]
    error: Option<Remote>,
}
#[derive(Deserialize)]
struct Remote {
    code: String,
    message: String,
}
fn remote_error<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Remote>, D::Error> {
    Remote::deserialize(d).map(Some)
}
pub(super) fn handshake(value: &Value) -> Result<(String, Snapshot, bool), Error> {
    let id = value["sessionId"]
        .as_str()
        .filter(|id| catalog::text(id, 384))
        .ok_or(Error::Wire)?;
    let features = value["features"]
        .as_array()
        .filter(|features| features.len() <= 32)
        .ok_or(Error::Wire)?;
    if features
        .iter()
        .any(|f| !f.as_str().is_some_and(|s| catalog::text(s, 128)))
    {
        return Err(Error::Wire);
    }
    if [
        "contribution.snapshot",
        "command.invoke",
        "resource.authorization.v1",
    ]
    .iter()
    .any(|f| !features.iter().any(|v| v == f))
    {
        return Err(Error::Unsupported);
    }
    let bytes =
        serde_json::to_vec(value.get("snapshot").ok_or(Error::Wire)?).map_err(|_| Error::Wire)?;
    Ok((
        id.into(),
        Snapshot::parse(&bytes).map_err(Error::Snapshot)?,
        features.iter().any(|f| f == "ocr-text.v1"),
    ))
}
fn response(
    value: &str,
    expected: &str,
    snapshot: &mut Option<Arc<Snapshot>>,
) -> Result<Option<Value>, Error> {
    let envelope: Value = serde_json::from_str(value).map_err(|_| Error::Wire)?;
    if !envelope.is_object() {
        return Err(Error::Wire);
    }
    if envelope["protocol"] == catalog::PROTOCOL
        && envelope["method"] == "loom.extension.snapshot.updated"
    {
        if envelope["apiVersion"] != catalog::API {
            return Err(Error::Unsupported);
        }
        if let Some(previous) = snapshot.as_ref() {
            let next = Snapshot::parse(
                &serde_json::to_vec(&envelope["params"]["snapshot"]).map_err(|_| Error::Wire)?,
            )
            .map_err(Error::Snapshot)?;
            if next.generation() > previous.generation() {
                *snapshot = Some(Arc::new(next));
            }
        }
        return Ok(None);
    }
    if envelope["requestId"] != expected {
        return Ok(None);
    }
    let response: Response = serde_json::from_str(value).map_err(|_| Error::Wire)?;
    if response.protocol != catalog::PROTOCOL
        || response.api_version != catalog::API
        || !catalog::text(&response.request_id, 384)
        || !matches!(response.status.as_str(), "succeeded" | "failed")
    {
        return Err(Error::Wire);
    }
    if let Some(error) = response.error.as_ref() {
        if !catalog::text(&error.code, 128) || !catalog::text(&error.message, 4096) {
            return Err(Error::Wire);
        }
    }
    if response.status == "failed" {
        return match response.error {
            Some(error) => Err(Error::Remote {
                code: error.code,
                message: error.message,
            }),
            None => Err(Error::Denied),
        };
    }
    Ok(Some(response.data))
}
pub(super) fn exchange(
    socket: &mut BridgeSocket,
    snapshot: &mut Option<Arc<Snapshot>>,
    id: &str,
    request: Value,
    timeout: Duration,
    cancel: &AtomicBool,
) -> Result<Value, Error> {
    let deadline = Instant::now() + timeout;
    exchange_text(socket, snapshot, id, request.to_string(), deadline, cancel)
}
pub(super) fn exchange_text(
    socket: &mut BridgeSocket,
    snapshot: &mut Option<Arc<Snapshot>>,
    id: &str,
    request: String,
    deadline: Instant,
    cancel: &AtomicBool,
) -> Result<Value, Error> {
    if request.len() > MAX_BYTES {
        return Err(Error::Limit);
    }
    remaining(socket, deadline, Duration::from_secs(3), cancel)?;
    socket
        .send(Message::Text(request))
        .map_err(|_| transport_error(cancel))?;
    loop {
        if let Some(data) = receive(
            socket,
            snapshot,
            id,
            deadline,
            Duration::from_millis(250),
            cancel,
        )? {
            return Ok(data);
        }
    }
}

pub(super) fn wait_update(
    socket: &mut BridgeSocket,
    snapshot: &mut Option<Arc<Snapshot>>,
    cancel: &AtomicBool,
) -> Result<(), Error> {
    let generation = snapshot.as_ref().ok_or(Error::Closed)?.generation();
    loop {
        // 空闲不是 RPC 超时；这只是有界单次读预算，不发送保活或触发 UI timer。
        let timeout = Duration::from_secs(30);
        receive(
            socket,
            snapshot,
            "",
            Instant::now() + timeout,
            timeout,
            cancel,
        )?;
        if snapshot
            .as_ref()
            .is_some_and(|next| next.generation() != generation)
        {
            return Ok(());
        }
    }
}
fn receive(
    socket: &mut BridgeSocket,
    snapshot: &mut Option<Arc<Snapshot>>,
    id: &str,
    deadline: Instant,
    read_cap: Duration,
    cancel: &AtomicBool,
) -> Result<Option<Value>, Error> {
    remaining(socket, deadline, read_cap, cancel)?;
    match socket.read() {
        Ok(Message::Text(text)) => {
            let value = response(&text, id, snapshot)?;
            remaining(socket, deadline, read_cap, cancel)?;
            Ok(value)
        }
        Ok(Message::Ping(_)) => {
            remaining(socket, deadline, Duration::from_secs(1), cancel)?;
            socket.flush().map_err(|_| transport_error(cancel))?;
            Ok(None)
        }
        Ok(Message::Close(_)) => Err(transport_error(cancel)),
        Ok(_) => Ok(None),
        Err(tungstenite::Error::Io(error))
            if matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            Ok(None)
        }
        Err(tungstenite::Error::Capacity(_)) => Err(Error::Limit),
        Err(_) => Err(transport_error(cancel)),
    }
}
