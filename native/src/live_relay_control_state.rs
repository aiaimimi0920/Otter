//! 来自已认证控制流的 session_state；reason 是可选开放字符串，不假造移除原因枚举。
use super::*;
use crate::live_relay_viewer_identity::identifier;
use serde::Deserialize;
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    Visible,
    Unfocused,
    LogicalMinimized,
    LogicalHidden,
    Restoring,
    CaptureRecovering,
    CaptureFailed,
    Closed,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct State {
    revision: u64,
    visibility: Visibility,
    viewers: Vec<String>,
    controller_device_id: Option<String>,
    reason: Option<String>,
}
pub(super) struct Reduced {
    pub revision: u64,
    pub visibility: Option<Visibility>,
    pub controller: Option<String>,
    pub closed: bool,
}
pub(super) fn reduce(
    events: &[ControlEvent],
    device: &str,
    revision: u64,
    initial: bool,
    controller: Option<String>,
    visibility: Option<Visibility>,
) -> Result<Reduced, Error> {
    let mut result = Reduced {
        revision,
        visibility,
        controller,
        closed: false,
    };
    if initial && events.is_empty() {
        return Err(Error::Protocol);
    }
    for (index, event) in events.iter().enumerate() {
        if event.kind() != "session_state" {
            if initial && index == 0 {
                return Err(Error::Protocol);
            }
            continue;
        }
        let state: State = serde_json::from_str(event.payload()).map_err(|_| Error::Protocol)?;
        if state.revision < result.revision
            || state.revision == 0
            || state.viewers.iter().any(|id| !identifier(id))
            || state
                .controller_device_id
                .as_ref()
                .is_some_and(|id| !identifier(id))
        {
            return Err(Error::Protocol);
        }
        if initial
            && index == 0
            && (state.revision != revision
                || state.reason.as_deref() != Some("viewer_joined")
                || !state.viewers.iter().any(|id| id == device))
        {
            return Err(Error::Protocol);
        }
        result.closed |=
            state.visibility == Visibility::Closed || !state.viewers.iter().any(|id| id == device);
        result.revision = state.revision;
        result.visibility = Some(state.visibility);
        result.controller = state.controller_device_id;
    }
    Ok(result)
}
