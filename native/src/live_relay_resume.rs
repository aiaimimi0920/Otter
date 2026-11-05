//! 显式控制恢复；仅成功且身份/成员匹配时交付新序列，不从 409 正文猜测游标。
use super::*;
pub struct Resumed {
    pub snapshot: Snapshot,
    pub control_sequence: u64,
}
impl Control<'_> {
    pub fn resume(
        &self,
        expected: &Identity,
        baseline: RequesterCursor,
        last_frame: u64,
    ) -> Result<Resumed, Error> {
        if baseline.input >= crate::live_input::MAX_SEQUENCE {
            return Err(Error::Cursor);
        }
        let sequence = baseline.next_control()?;
        let body = serde_json::to_vec(&serde_json::json!({"envelope":{
            "protocolVersion":protocol::PROTOCOL,"sessionId":expected.session(),
            "epoch":expected.epoch(),"sequence":sequence,"messageType":"resume_request",
            "payload":{"lastControlSequence":baseline.control,"lastFrameId":last_frame,
                "lastInputSequence":baseline.input,"requesterDeviceId":self.session.device_id()}
        }}))
        .map_err(|_| Error::Input)?;
        let bytes = self.send(
            "POST",
            &path(expected.session(), Some("resume"))?,
            &body,
            Duration::from_secs(10),
        )?;
        let snapshot = Snapshot::parse(&bytes, expected.session()).map_err(Error::Protocol)?;
        snapshot
            .renewal(expected, self.session.device_id())
            .map_err(Error::Protocol)?;
        Ok(Resumed {
            snapshot,
            control_sequence: sequence,
        })
    }
}
