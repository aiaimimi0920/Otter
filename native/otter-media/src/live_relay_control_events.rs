//! Two-phase control-stream validation: prepare the complete batch, then commit the cursor only after business validation/application succeeds.
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
    /// after comes from a verified join_cursor or an event position committed by this owner, never from a media packet.
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
    /// The session owner must apply business results and commit within the same transaction/lock; prepare performs no event side effects.
    /// On failure, discard the batch and retry; an old batch cannot overwrite an advanced position or be passed to another cursor.
    pub fn commit(&mut self, batch: &ControlBatch) -> Result<(), Error> {
        self.validate(batch)?;
        self.after = batch.next;
        Ok(())
    }
    /// Recheck the batch owner/start before applying side effects; the same exclusive control owner must still perform the actual commit.
    pub fn validate(&self, batch: &ControlBatch) -> Result<(), Error> {
        if !Arc::ptr_eq(&self.owner, &batch.owner) || self.after != batch.after {
            return Err(Error::History);
        }
        Ok(())
    }
}
