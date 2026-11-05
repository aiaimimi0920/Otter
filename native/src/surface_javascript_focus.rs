//! 私有、逐 operation 的焦点请求；不接受 HWND、路径、远端动作或激活权限。
use crate::surface_javascript_wire as wire;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub target: String,
    pub focus: bool,
}
pub(crate) fn validate(requests: &[Request]) -> std::io::Result<()> {
    if requests.len() > 32
        || requests
            .iter()
            .any(|item| !crate::surface_javascript_event::safe(&item.target))
    {
        return Err(wire::invalid());
    }
    Ok(())
}
