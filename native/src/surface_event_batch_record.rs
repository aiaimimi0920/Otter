//! 批次退出提示：仅保存有序身份和阶段，不保存可执行 payload、提示或场景。
use super::*;
use crate::{
    desktop::surface_authorized_window::BatchReceipt,
    live_relay_http::BatchSubmission,
    surface_event_batch_runtime::{Error as BatchError, Observation, Outcome as BatchOutcome},
};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BatchHint {
    first_event_id: String,
    index: usize,
    count: usize,
    presentation: Stage,
}
impl Record {
    pub fn from_batch_outcome(outcome: &BatchOutcome) -> io::Result<Vec<Self>> {
        records(
            &outcome.input.events,
            &outcome.submission,
            &outcome.observation,
            Stage::NotStarted,
        )
    }
    pub fn from_batch_receipt(receipt: &BatchReceipt) -> io::Result<Vec<Self>> {
        let stage = match &receipt.presentation {
            Ok(Some(revision)) => Stage::Succeeded(*revision),
            Ok(None)
            | Err(crate::desktop::surface_authorized_window::PresentationError::Pending(_)) => {
                Stage::NotStarted
            }
            Err(_) => Stage::Failed,
        };
        records(
            &receipt.events,
            &receipt.submission,
            &receipt.observation,
            stage,
        )
    }
}
fn records(
    events: &[ClickEvent],
    submission: &Result<BatchSubmission, BatchError>,
    observation: &Option<Result<Observation, BatchError>>,
    presentation: Stage,
) -> io::Result<Vec<Record>> {
    let first = events.first().ok_or_else(invalid)?;
    if events.len() > MAX_RECORDS {
        return Err(invalid());
    }
    let acks = submission
        .as_ref()
        .ok()
        .map(|value| value.acknowledgements.as_slice())
        .unwrap_or_default();
    let items = observation
        .as_ref()
        .and_then(|value| value.as_ref().ok())
        .map(|value| value.items.as_slice())
        .unwrap_or_default();
    if acks.len() > events.len() || items.len() > acks.len() {
        return Err(invalid());
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(events.len())
        .map_err(io::Error::other)?;
    for (index, event) in events.iter().enumerate() {
        let ack = acks.get(index);
        if event.binding() != first.binding() || ack.is_some_and(|ack| !ack.matches_event(event)) {
            return Err(invalid());
        }
        let item = items.get(index);
        let convergence = match item {
            Some(item) => match item.result {
                Ok(revision) => Stage::Succeeded(revision),
                Err(_) => Stage::Failed,
            },
            None if ack.is_some() && matches!(observation, Some(Err(_))) => Stage::Failed,
            None => Stage::NotStarted,
        };
        // 只有最高候选版本的快照真正被应用；其他项的整体呈现状态记录在 batch 中。
        let applied = match &presentation {
            Stage::Succeeded(revision) if convergence == Stage::Succeeded(*revision) => {
                presentation.clone()
            }
            Stage::Failed => Stage::Failed,
            _ => Stage::NotStarted,
        };
        let mut record = Record::new(event, ack, convergence, applied);
        record.result_revision = item
            .and_then(|item| item.commit.as_ref())
            .map(|commit| commit.revision());
        record.decision = item
            .and_then(|item| item.decision.as_ref())
            .map(|evidence| DecisionHint::from_decision(&evidence.decision));
        record.batch = Some(BatchHint {
            first_event_id: first.id().into(),
            index,
            count: events.len(),
            presentation: presentation.clone(),
        });
        record.validate()?;
        output.push(record);
    }
    validate_groups(&output)?;
    Ok(output)
}
pub(super) fn validate_groups(records: &[Record]) -> io::Result<()> {
    for record in records {
        let Some(batch) = &record.batch else { continue };
        if batch.count == 0
            || batch.count > MAX_RECORDS
            || batch.index >= batch.count
            || !crate::live_relay_viewer_identity::identifier(&batch.first_event_id)
        {
            return Err(invalid());
        }
        let members: Vec<_> = records
            .iter()
            .filter(|other| {
                other
                    .batch
                    .as_ref()
                    .is_some_and(|hint| hint.first_event_id == batch.first_event_id)
            })
            .collect();
        if members.len() != batch.count {
            return Err(invalid());
        }
        for (index, member) in members.iter().enumerate() {
            let hint = member.batch.as_ref().expect("filtered batch");
            if hint.index != index
                || hint.count != batch.count
                || hint.presentation != batch.presentation
                || member.attachment != record.attachment
                || member.base_revision != record.base_revision
                || (index == 0 && member.event_id != batch.first_event_id)
                || members[..index]
                    .iter()
                    .any(|prior| prior.event_id == member.event_id)
            {
                return Err(invalid());
            }
            if let Stage::Succeeded(revision) = batch.presentation {
                if !matches!(member.convergence, Stage::Succeeded(value) if value <= revision)
                    || revision < member.base_revision
                {
                    return Err(invalid());
                }
            }
        }
        if let Stage::Succeeded(revision) = batch.presentation {
            if !members
                .iter()
                .any(|member| member.convergence == Stage::Succeeded(revision))
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}
