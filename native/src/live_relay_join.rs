//! 发现→actor 专属游标→加入→历史起点；准备完成仍不代表控制历史已应用或媒体可发布。
use super::*;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequesterCursor {
    pub control: u64,
    pub input: u64,
}
impl RequesterCursor {
    pub fn next_control(self) -> Result<u64, Error> {
        self.control.checked_add(1).ok_or(Error::Cursor)
    }
}
pub struct Join<'a> {
    pub session: &'a str,
    pub surface: &'a str,
    pub attachment: &'a str,
    /// Some 表示旧观看的续签；不得因旧成员已移除而静默重新加入。
    pub expected: Option<&'a Identity>,
}
pub struct Prepared {
    pub business: crate::live_relay_viewer_business::Business,
    pub snapshot: Snapshot,
    pub cursor: ControlCursor,
    pub control_sequence: u64,
    pub input_sequence: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Discovery {
    protocol_version: String,
    sessions: Vec<Box<serde_json::value::RawValue>>,
}
#[derive(Deserialize)]
struct Entry {
    session: Header,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Header {
    session_id: String,
    viewer_devices: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recovery {
    requester_control: ActorCursor,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActorCursor {
    device_id: String,
    epoch: u64,
    control_sequence: u64,
    input_sequence: u64,
}

impl Control<'_> {
    pub fn discover(&self, session: &str) -> Result<Snapshot, Error> {
        path(session, None)?;
        let bytes = self.send("GET", "/v1/live/sessions", &[], Duration::from_secs(10))?;
        discovery(&bytes, session)
    }
    pub fn join_baseline(&self, preview: &Snapshot) -> Result<RequesterCursor, Error> {
        if preview
            .renewal(preview.identity(), self.session.device_id())
            .is_err()
        {
            return Ok(RequesterCursor {
                control: 0,
                input: 0,
            });
        }
        let bytes = self.send(
            "GET",
            &path(preview.identity().session(), None)?,
            &[],
            Duration::from_secs(10),
        )?;
        recovery(&bytes, preview, self.session.device_id())
    }
    pub fn prepare_viewer(&self, request: Join<'_>) -> Result<Prepared, Error> {
        self.prepare(request, None)
    }
    /// 显式从目录选择允许首次加入，但目录身份必须在请求前后保持一致。
    pub fn prepare_selected_viewer(
        &self,
        request: Join<'_>,
        selected: &Identity,
    ) -> Result<Prepared, Error> {
        self.prepare(request, Some(selected))
    }
    fn prepare(&self, request: Join<'_>, selected: Option<&Identity>) -> Result<Prepared, Error> {
        // 在第一次网络请求前验证全部用户输入；无效选择不得产生远端加入副作用。
        path(request.session, None)?;
        if !protocol::identifier(request.surface)
            || !protocol::identifier(request.attachment)
            || request
                .expected
                .is_some_and(|id| id.session() != request.session)
            || selected.is_some_and(|id| {
                id.session() != request.session
                    || request.expected.is_some_and(|expected| expected != id)
            })
        {
            return Err(Error::Input);
        }
        self.prepare_bound(
            request.session,
            request.expected,
            selected,
            |identity, sequence| {
                self.attach_business(
                    identity,
                    Attach {
                        surface: request.surface,
                        attachment: request.attachment,
                        sequence,
                        require_existing_membership: request.expected.is_some(),
                    },
                )
            },
        )
    }
    pub(crate) fn prepare_local_art_viewer(
        &self,
        session: &str,
        selected: &Identity,
        proof: &art_prepare::viewer::Proof,
    ) -> Result<Prepared, Error> {
        path(session, None)?;
        if selected.session() != session {
            return Err(Error::Input);
        }
        proof.validate()?;
        self.prepare_bound(session, None, Some(selected), |identity, sequence| {
            self.attach_local_art(identity, proof, sequence, false)
        })
    }
    fn prepare_bound(
        &self,
        session: &str,
        expected: Option<&Identity>,
        selected: Option<&Identity>,
        attach: impl FnOnce(
            &Identity,
            u64,
        )
            -> Result<(Snapshot, crate::live_relay_viewer_business::Business), Error>,
    ) -> Result<Prepared, Error> {
        let preview = self.discover(session)?;
        if let Some(selected) = selected {
            preview.matches(selected).map_err(Error::Protocol)?;
        }
        if let Some(expected) = expected {
            preview
                .renewal(expected, self.session.device_id())
                .map_err(Error::Protocol)?;
        }
        let baseline = self.join_baseline(&preview)?;
        let sequence = baseline.next_control()?;
        let (attached, business) = attach(preview.identity(), sequence)?;
        if attached.revision() < preview.revision() {
            return Err(Error::Protocol(protocol::Error::Revision));
        }
        let after = self.bootstrap_cursor(&attached)?;
        Ok(Prepared {
            business,
            cursor: ControlCursor::new(attached.identity().clone(), after),
            snapshot: attached,
            control_sequence: sequence,
            input_sequence: baseline.input,
        })
    }
}
fn discovery(bytes: &[u8], session: &str) -> Result<Snapshot, Error> {
    let response: Discovery = protocol::parse(bytes).map_err(Error::Protocol)?;
    if response.protocol_version != protocol::PROTOCOL || response.sessions.len() > 512 {
        return Err(Error::Discovery);
    }
    let mut found = None;
    for raw in response.sessions {
        // viewerDevices 必须存在；缺字段不能被当成空成员列表，从而跳过序列恢复。
        let entry: Entry = serde_json::from_str(raw.get()).map_err(|_| Error::Discovery)?;
        if entry.session.session_id != session {
            continue;
        }
        if found.is_some()
            || entry
                .session
                .viewer_devices
                .iter()
                .any(|id| !protocol::identifier(id))
        {
            return Err(Error::Discovery);
        }
        found = Some(Snapshot::parse(raw.get().as_bytes(), session).map_err(Error::Protocol)?);
    }
    found.ok_or(Error::Discovery)
}
pub(super) fn recovery(
    bytes: &[u8],
    preview: &Snapshot,
    device: &str,
) -> Result<RequesterCursor, Error> {
    let snapshot = Snapshot::parse(bytes, preview.identity().session()).map_err(Error::Protocol)?;
    snapshot
        .renewal(preview.identity(), device)
        .map_err(Error::Protocol)?;
    if snapshot.revision() < preview.revision() {
        return Err(Error::Cursor);
    }
    let response: Recovery = protocol::parse(bytes).map_err(Error::Protocol)?;
    let actor = response.requester_control;
    if actor.device_id != device
        || actor.epoch != preview.identity().epoch()
        || actor.input_sequence >= crate::live_input::MAX_SEQUENCE
    {
        return Err(Error::Cursor);
    }
    let cursor = RequesterCursor {
        control: actor.control_sequence,
        input: actor.input_sequence,
    };
    cursor.next_control()?;
    Ok(cursor)
}

#[cfg(test)]
#[path = "live_relay_join_tests.rs"]
mod tests;
