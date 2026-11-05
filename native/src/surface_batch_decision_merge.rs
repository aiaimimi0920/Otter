//! 决定结果并回原批次，只覆盖目标观察项；不补发未提交后缀、不复制 owner。
use super::*;
use crate::{
    live_relay_http::BatchStop,
    surface_action_ack::Status,
    surface_decision_runtime::Evidence,
    surface_event_batch_runtime::{Input as BatchInput, ObservationError, Outcome as BatchOutcome},
};
impl Outcome {
    /// 不一致时完整返还，调用方必须保存错误输入而不是丢弃决定或其他事件。
    pub fn into_batch(self) -> Result<BatchOutcome, Box<Self>> {
        let prepared = &self.input.prepared;
        let receipt = &prepared.receipt;
        let valid =
            receipt.events.get(prepared.index).is_some_and(|event| {
                receipt.submission.as_ref().ok().is_some_and(|submitted| {
                    submitted.acknowledgements.len() <= receipt.events.len()
                        && submitted
                            .acknowledgements
                            .get(prepared.index)
                            .is_some_and(|ack| prepared.decision.matches(event, ack))
                })
            }) && receipt
                .observation
                .as_ref()
                .and_then(|value| value.as_ref().ok())
                .is_some_and(|observed| {
                    receipt.submission.as_ref().is_ok_and(|submitted| {
                        observed.items.len() == submitted.acknowledgements.len()
                    }) && observed
                        .items
                        .get(prepared.index)
                        .is_some_and(|item| item.decision.is_none() && item.confirmation.is_none())
                });
        if !valid {
            return Err(Box::new(self));
        }
        let Outcome {
            input,
            submission,
            convergence,
            delivery,
        } = self;
        let Input {
            owner,
            revision,
            prepared,
        } = input;
        let crate::desktop::surface_authorized_window::PreparedBatchDecision {
            mut receipt,
            index,
            decision,
        } = prepared;
        let observed = receipt
            .observation
            .as_mut()
            .expect("validated observation")
            .as_mut()
            .expect("validated result");
        let item = &mut observed.items[index];
        if let Some(convergence) = convergence {
            match convergence {
                Err(error) => item.result = Err(ObservationError::Decision(error)),
                Ok(mut snapshot) => {
                    item.result = Ok(snapshot.revision);
                    item.commit = snapshot.commit.take();
                    if observed
                        .snapshot
                        .as_ref()
                        .is_none_or(|current| current.revision <= snapshot.revision)
                    {
                        observed.snapshot = Some(snapshot);
                    }
                }
            }
        }
        item.decision = Some(Evidence {
            decision,
            revision,
            submission,
        });
        let submitted = receipt.submission.as_mut().expect("validated submission");
        if submitted.acknowledgements.len() == receipt.events.len()
            && submitted.acknowledgements.iter().all(|ack| ack.succeeded())
        {
            submitted.stop = BatchStop::Complete;
        } else if submitted.acknowledgements.iter().any(|ack| {
            matches!(
                ack.status(),
                Status::Cancelled | Status::Failed | Status::Interrupted | Status::Unknown
            )
        }) {
            submitted.stop = BatchStop::Action;
        }
        Ok(BatchOutcome {
            input: BatchInput {
                owner,
                revision,
                events: receipt.events,
            },
            submission: receipt.submission,
            observation: receipt.observation,
            delivery,
        })
    }
}
