//! 观看端只镜像 Loom 的执行审计，不在收到 trigger_event 时重复执行 Surface 动作。
use crate::{
    live_relay_observation::Source,
    live_relay_viewer_identity::{identifier, parse},
};
use serde::Deserialize;
use std::{collections::VecDeque, sync::Arc};
pub const MAX_AUDITS: usize = 256;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Json,
    Invalid,
    Limit,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Fired,
    Skipped,
    Failed,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Audit {
    pub trigger_id: String,
    pub binding_id: String,
    pub condition_revision: u64,
    pub observation_id: String,
    pub observation_sequence: u64,
    pub source_device_id: String,
    pub observation_source: Source,
    pub idempotency_key: String,
    pub outcome: Outcome,
    pub evaluated_at_ms: u64,
    pub authorized_by: String,
    pub action_request_id: Option<String>,
    pub reason: Option<String>,
}
impl Audit {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let value: Self = parse(bytes).map_err(|_| Error::Json)?;
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), Error> {
        if [
            &self.trigger_id,
            &self.binding_id,
            &self.observation_id,
            &self.source_device_id,
            &self.idempotency_key,
            &self.authorized_by,
        ]
        .iter()
        .any(|id| !identifier(id))
            || self.condition_revision == 0
            || self.observation_sequence == 0
            || self.evaluated_at_ms == 0
            || self
                .action_request_id
                .as_ref()
                .is_some_and(|id| !identifier(id))
            || self
                .reason
                .as_ref()
                .is_some_and(|reason| reason.len() > 512)
        {
            return Err(Error::Invalid);
        }
        Ok(())
    }
    fn reservation(&self) -> bool {
        self.action_request_id.is_none()
            && self.reason.as_deref() == Some("surface_action_dispatch_reserved")
    }
}
#[derive(Clone, Default)]
pub struct Audits {
    values: VecDeque<Arc<Audit>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    // 与主干兼容：旧服务可省略审计快照，但 null/错误类型仍失败。
    #[serde(default)]
    trigger_audits: Vec<Audit>,
}
impl Audits {
    pub fn from_snapshot(bytes: &[u8]) -> Result<Self, Error> {
        let snapshot: Snapshot = parse(bytes).map_err(|_| Error::Json)?;
        if snapshot.trigger_audits.len() > MAX_AUDITS {
            return Err(Error::Limit);
        }
        let mut state = Self::default();
        for value in snapshot.trigger_audits {
            value.validate()?;
            state.upsert(value);
        }
        Ok(state)
    }
    pub fn iter(&self) -> impl Iterator<Item = &Audit> {
        self.values.iter().map(AsRef::as_ref)
    }
    /// 完整业务事务应在其副本上调用本方法；这里只提交审计子批次。
    pub fn apply<'a>(&mut self, payloads: impl IntoIterator<Item = &'a [u8]>) -> Result<(), Error> {
        let mut next = self.clone();
        for payload in payloads {
            next.upsert(Audit::parse(payload)?);
        }
        *self = next;
        Ok(())
    }
    fn upsert(&mut self, value: Audit) {
        if let Some(existing) = self
            .values
            .iter_mut()
            .find(|old| old.idempotency_key == value.idempotency_key)
        {
            // 相同键的最终结果可替换 reservation；反方向即使迟到也必须忽略。
            if !existing.reservation() && value.reservation() {
                return;
            }
            if existing.as_ref() != &value {
                *existing = Arc::new(value);
            }
        } else {
            self.values.push_back(Arc::new(value));
            if self.values.len() > MAX_AUDITS {
                self.values.pop_front();
            }
        }
    }
}
#[cfg(test)]
#[path = "live_relay_trigger_audit_tests.rs"]
mod tests;
