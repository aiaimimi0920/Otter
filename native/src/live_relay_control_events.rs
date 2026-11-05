//! 控制流两阶段校验：先准备完整批次，业务校验/应用成功后才提交游标。
use super::{parse, Error, Identity, PROTOCOL};
use serde::Deserialize;
use std::sync::Arc;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Response {
    pub(super) protocol_version: String,
    pub(super) next: u64,
    #[serde(rename = "reset")]
    reset: bool,
    pub(super) events: Vec<ControlEvent>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlEvent {
    pub(super) protocol_version: String,
    pub(super) session_id: String,
    pub(super) epoch: u64,
    pub(super) sequence: u64,
    pub(super) message_type: String,
    pub(super) payload: Box<serde_json::value::RawValue>,
}
impl ControlEvent {
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn kind(&self) -> &str {
        &self.message_type
    }
    pub fn payload(&self) -> &str {
        self.payload.get()
    }
}
pub struct ControlBatch {
    owner: Arc<()>,
    after: u64,
    next: u64,
    events: Vec<ControlEvent>,
}
impl ControlBatch {
    pub fn events(&self) -> &[ControlEvent] {
        &self.events
    }
    pub fn next(&self) -> u64 {
        self.next
    }
}
pub struct ControlCursor {
    owner: Arc<()>,
    identity: Identity,
    after: u64,
}
impl ControlCursor {
    /// after 来自已验证的 join_cursor 或本 owner 已提交的事件位置，不从媒体包取值。
    pub fn new(identity: Identity, after: u64) -> Self {
        Self {
            owner: Arc::new(()),
            identity,
            after,
        }
    }
    pub fn after(&self) -> u64 {
        self.after
    }
    pub fn prepare(&self, bytes: &[u8]) -> Result<ControlBatch, Error> {
        let response: Response = parse(bytes)?;
        if response.reset || response.protocol_version != PROTOCOL || response.next < self.after {
            return Err(Error::History);
        }
        let mut next = self.after;
        for event in &response.events {
            if event.protocol_version != PROTOCOL
                || event.session_id != self.identity.session()
                || event.epoch != self.identity.epoch()
                || next.checked_add(1) != Some(event.sequence)
            {
                return Err(Error::History);
            }
            next = event.sequence;
        }
        if next != response.next {
            return Err(Error::History);
        }
        Ok(ControlBatch {
            owner: self.owner.clone(),
            after: self.after,
            next,
            events: response.events,
        })
    }
    /// 会话 owner 须在同一事务/锁内应用业务结果并提交；prepare 不执行任何事件副作用。
    /// 失败可丢弃 batch 重试；旧批次不能覆盖已推进的位置，也不能交给另一 cursor。
    pub fn commit(&mut self, batch: &ControlBatch) -> Result<(), Error> {
        self.validate(batch)?;
        self.after = batch.next;
        Ok(())
    }
    /// 应用副作用前复核批次 owner/起点；实际提交仍须同一独占控制 owner 执行。
    pub fn validate(&self, batch: &ControlBatch) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.owner, &batch.owner) || self.after != batch.after {
            return Err(Error::History);
        }
        Ok(())
    }
}
