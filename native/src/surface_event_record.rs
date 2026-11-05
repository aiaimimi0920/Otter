//! 动作恢复提示，不是授权或可重放命令；不保存 payload、场景、凭据和远端错误文本。
use crate::{
    desktop::surface_authorized_window::EventReceipt,
    surface_action_ack::{Ack, Status},
    surface_click_event::ClickEvent,
    surface_event_runtime::Outcome,
    surface_recovery_record::Record as Attachment,
};
use serde::{Deserialize, Serialize};
use std::io;
pub const MAX_BYTES: usize = 64 * 1024;
pub const MAX_RECORDS: usize = 32;
#[path = "surface_event_batch_record.rs"]
mod batch;
pub use batch::BatchHint;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Record {
    attachment: Attachment,
    event_id: String,
    base_revision: u64,
    attempted: bool,
    ack: Option<AckHint>,
    convergence: Stage,
    presentation: Stage,
    result_revision: Option<u64>,
    decision: Option<DecisionHint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    batch: Option<BatchHint>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionHint {
    confirmation_id: String,
    approved: bool,
    attempted: bool,
    confirmed: bool,
}
impl DecisionHint {
    fn from_decision(decision: &crate::surface_confirmation::Decision) -> Self {
        Self {
            confirmation_id: decision.request().id().into(),
            approved: decision.choice() == crate::surface_confirmation::Choice::Approve,
            attempted: decision.attempted(),
            confirmed: decision.confirmed(),
        }
    }
    pub fn confirmation_id(&self) -> &str {
        &self.confirmation_id
    }
    pub fn approved(&self) -> bool {
        self.approved
    }
    pub fn attempted(&self) -> bool {
        self.attempted
    }
    pub fn confirmed(&self) -> bool {
        self.confirmed
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AckHint {
    request_id: String,
    accepted: bool,
    status: Status,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    content = "revision",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum Stage {
    NotStarted,
    Failed,
    Succeeded(u64),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u32,
    records: Vec<Record>,
}
impl Record {
    fn new(event: &ClickEvent, ack: Option<&Ack>, convergence: Stage, presentation: Stage) -> Self {
        Self {
            attachment: Attachment::from_binding(event.binding(), None, false, false),
            event_id: event.id().into(),
            base_revision: event.revision(),
            attempted: event.attempted(),
            ack: ack.map(|ack| AckHint {
                request_id: ack.request_id().into(),
                accepted: ack.accepted(),
                status: ack.status(),
            }),
            convergence,
            presentation,
            result_revision: None,
            decision: None,
            batch: None,
        }
    }
    pub fn from_outcome(outcome: &Outcome) -> Self {
        let mut record = Self::new(
            &outcome.event,
            outcome.result.as_ref().ok(),
            match &outcome.convergence {
                None => Stage::NotStarted,
                Some(Err(_)) => Stage::Failed,
                Some(Ok(snapshot)) => Stage::Succeeded(snapshot.revision),
            },
            Stage::NotStarted,
        );
        record.decision = outcome
            .decision
            .as_ref()
            .map(|evidence| DecisionHint::from_decision(&evidence.decision));
        record.result_revision = outcome
            .convergence
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .and_then(|snapshot| snapshot.commit.as_ref())
            .map(|commit| commit.revision());
        record
    }
    pub fn from_receipt(receipt: &EventReceipt) -> Self {
        let mut record = Self::new(
            &receipt.event,
            receipt.result.as_ref().ok(),
            match receipt.convergence {
                None => Stage::NotStarted,
                Some(Err(_)) => Stage::Failed,
                Some(Ok(revision)) => Stage::Succeeded(revision),
            },
            match &receipt.presentation {
                Ok(None)
                | Err(crate::desktop::surface_authorized_window::PresentationError::Pending(_)) => {
                    Stage::NotStarted
                }
                Err(_) => Stage::Failed,
                Ok(Some(revision)) => Stage::Succeeded(*revision),
            },
        );
        record.result_revision = receipt.commit.as_ref().map(|commit| commit.revision());
        record.decision = receipt
            .decision
            .as_ref()
            .map(|evidence| DecisionHint::from_decision(&evidence.decision));
        record
    }
    pub fn from_decision(outcome: &crate::surface_decision_runtime::Outcome) -> Self {
        let mut record = Self::new(
            &outcome.input.event,
            Some(&outcome.input.ack),
            match &outcome.convergence {
                None => Stage::NotStarted,
                Some(Err(_)) => Stage::Failed,
                Some(Ok(snapshot)) => Stage::Succeeded(snapshot.revision),
            },
            Stage::NotStarted,
        );
        let decision = &outcome.input.decision;
        record.decision = Some(DecisionHint::from_decision(decision));
        record.result_revision = outcome
            .convergence
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .and_then(|snapshot| snapshot.commit.as_ref())
            .map(|commit| commit.revision());
        record
    }
    pub fn decision(&self) -> Option<&DecisionHint> {
        self.decision.as_ref()
    }
    pub fn event_id(&self) -> &str {
        &self.event_id
    }
    pub fn attempted(&self) -> bool {
        self.attempted
    }
    pub fn attachment(&self) -> &Attachment {
        &self.attachment
    }
    pub fn status(&self) -> Option<Status> {
        self.ack.as_ref().map(|ack| ack.status)
    }
    pub fn convergence(&self) -> &Stage {
        &self.convergence
    }
    pub fn presentation(&self) -> &Stage {
        &self.presentation
    }
    fn validate(&self) -> io::Result<()> {
        self.attachment.validate()?;
        if !self.attachment.has_binding()
            || self.decision.as_ref().is_some_and(|decision| {
                !crate::live_relay_viewer_identity::identifier(&decision.confirmation_id)
                    || !self.attempted
                    || self.ack.is_none()
                    || (decision.confirmed && !decision.attempted)
                    || (matches!(self.convergence, Stage::Succeeded(_)) && !decision.confirmed)
            })
            || self.result_revision.is_some_and(|revision| {
                revision == 0 || !matches!(self.convergence, Stage::Succeeded(_))
            })
            || !crate::live_relay_viewer_identity::identifier(&self.event_id)
            || self.base_revision == 0
            || self.ack.as_ref().is_some_and(|ack| {
                !self.attempted || !crate::live_relay_viewer_identity::identifier(&ack.request_id)
            })
            || matches!(self.convergence, Stage::Succeeded(revision) if revision < self.base_revision)
            || matches!(self.presentation, Stage::Succeeded(revision)
                if self.convergence != Stage::Succeeded(revision))
            || (matches!(self.convergence, Stage::Succeeded(_))
                && !self
                    .ack
                    .as_ref()
                    .is_some_and(|ack| ack.accepted && ack.status == Status::Succeeded))
        {
            return Err(invalid());
        }
        Ok(())
    }
}
pub fn encode(records: &[Record]) -> io::Result<Vec<u8>> {
    if records.len() > MAX_RECORDS {
        return Err(invalid());
    }
    for record in records {
        record.validate()?;
    }
    batch::validate_groups(records)?;
    let bytes = serde_json::to_vec(&Envelope {
        version: 1,
        records: records.to_vec(),
    })
    .map_err(io::Error::other)?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> io::Result<Vec<Record>> {
    if bytes.len() > MAX_BYTES {
        return Err(invalid());
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(io::Error::other)?;
    if envelope.version != 1 || envelope.records.len() > MAX_RECORDS {
        return Err(invalid());
    }
    for record in &envelope.records {
        record.validate()?;
    }
    batch::validate_groups(&envelope.records)?;
    Ok(envelope.records)
}
fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid Surface event recovery metadata",
    )
}
#[cfg(test)]
#[path = "surface_event_record_tests.rs"]
mod tests;
