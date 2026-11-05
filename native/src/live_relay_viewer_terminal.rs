//! 权威 member snapshot 的关闭事实；HTTP 404/网络错误本身不等于会话关闭。
use super::{identifier, parse, Error, Identity, PROTOCOL};
use serde::Deserialize;
#[derive(Deserialize)]
struct Terminal {
    session: Session,
    epoch: u64,
    closed: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Session {
    protocol_version: String,
    session_id: String,
    viewer_devices: Vec<String>,
}
/// 仅对已认证 member endpoint 的成功响应调用；不接受媒体消息或无身份的 HTTP 错误正文。
pub fn terminal_snapshot(bytes: &[u8], expected: &Identity, device: &str) -> Result<bool, Error> {
    let response: Terminal = parse(bytes)?;
    if !identifier(device)
        || response.session.protocol_version != PROTOCOL
        || response.session.session_id != expected.session()
        || response.epoch < expected.epoch()
        || !response.session.viewer_devices.iter().any(|v| v == device)
    {
        return Err(Error::Identity);
    }
    Ok(response.closed)
}
