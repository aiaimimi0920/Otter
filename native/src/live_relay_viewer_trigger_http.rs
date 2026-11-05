//! 单次注册+只读精确对账；未知结果不重放，不把较新的业务快照覆盖可靠观察游标。
use super::*;
use crate::{
    live_relay_trigger_command::Command,
    live_relay_trigger_registration::{self as registration, Registration},
};
pub(crate) struct TriggerOutcome {
    pub accepted: bool,
    pub registrations: Vec<Registration>,
}
impl Control<'_> {
    pub(crate) fn trigger_snapshot(
        &self,
        identity: &Identity,
    ) -> Result<(RequesterCursor, Vec<Registration>), Error> {
        let bytes = self.send(
            "GET",
            &path(identity.session(), None)?,
            &[],
            Duration::from_secs(3),
        )?;
        let snapshot = Snapshot::parse(&bytes, identity.session()).map_err(Error::Protocol)?;
        snapshot
            .renewal(identity, self.session.device_id())
            .map_err(Error::Protocol)?;
        let cursor = join::recovery(&bytes, &snapshot, self.session.device_id())?;
        let registrations = registration::from_snapshot(&bytes).map_err(|_| Error::Input)?;
        Ok((cursor, registrations))
    }
    pub(crate) fn configure_trigger(
        &self,
        identity: &Identity,
        baseline: RequesterCursor,
        command: &Command,
    ) -> Result<TriggerOutcome, Error> {
        command.validate().map_err(|_| Error::Input)?;
        let next = baseline.next_control()?;
        if next > crate::live_input::MAX_SEQUENCE {
            return Err(Error::Cursor);
        }
        let (before, registrations) = self.trigger_snapshot(identity)?;
        if before != baseline {
            return Err(Error::Cursor);
        }
        let revision = registrations
            .iter()
            .find(|r| r.binding.binding_id == command.binding_id)
            .map_or(Some(1), |r| r.condition.revision.checked_add(1));
        if revision != Some(command.condition.revision) {
            return Ok(TriggerOutcome {
                accepted: false,
                registrations,
            });
        }
        let body = serde_json::to_vec(&serde_json::json!({
            "bindingId":command.binding_id,"enabled":command.enabled,"target":command.target,
            "envelope":{"protocolVersion":protocol::PROTOCOL,"sessionId":identity.session(),
                "epoch":identity.epoch(),"sequence":next,"messageType":"trigger_condition",
                "payload":command.condition}
        }))
        .map_err(|_| Error::Input)?;
        let result = self.send(
            "POST",
            &path(identity.session(), Some("triggers"))?,
            &body,
            Duration::from_secs(3),
        );
        // POST响应未填requesterControl；无论回复丢失与否，都只读本人游标和完整唯一注册。
        let (after, registrations) = self.trigger_snapshot(identity)?;
        let exact = registrations
            .iter()
            .any(|r| command.matches(r, self.session.device_id()));
        if after
            == (RequesterCursor {
                control: next,
                input: baseline.input,
            })
            && exact
        {
            return Ok(TriggerOutcome {
                accepted: true,
                registrations,
            });
        }
        if matches!(result, Err(Error::Status(400..=499))) && after == baseline {
            return Ok(TriggerOutcome {
                accepted: false,
                registrations,
            });
        }
        // 超时后看到旧cursor也不能证明在途POST永不提交；关闭旧观看，不安全复用actor。
        Err(result.err().unwrap_or(Error::Cursor))
    }
}
