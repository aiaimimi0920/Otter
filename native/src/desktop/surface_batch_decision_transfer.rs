//! 已明确选择的批次决定移交；当前窗口复核后转移唯一 owner，失败保存全部准备结果。
use super::*;
pub struct BatchDecisionInput {
    pub owner: Owner,
    pub revision: u64,
    pub prepared: PreparedBatchDecision,
}
pub struct BatchDecisionStartError {
    pub prepared: PreparedBatchDecision,
    pub error: Error,
}
impl AuthorizedWindow {
    pub fn begin_prepared_batch_decision(
        &mut self,
        prepared: PreparedBatchDecision,
        now_ms: u64,
    ) -> std::result::Result<BatchDecisionInput, Box<BatchDecisionStartError>> {
        let transfer = (|| {
            self.refresh(now_ms)?;
            let receipt = &prepared.receipt;
            if self.stopped.is_some()
                || !self.is_shown()
                || receipt.delivery.is_err()
                || receipt.revision != self.snapshot_revision
                || !matches!(
                    receipt.presentation,
                    Ok(None) | Err(PresentationError::Evidence)
                )
            {
                return Err(Error::new(
                    E_FAIL,
                    "Surface batch decision is not authorized",
                ));
            }
            let event = receipt
                .events
                .get(prepared.index)
                .ok_or_else(|| Error::new(E_FAIL, "Surface batch event missing"))?;
            let ack = receipt
                .submission
                .as_ref()
                .ok()
                .and_then(|value| value.acknowledgements.get(prepared.index))
                .ok_or_else(|| Error::new(E_FAIL, "Surface batch acknowledgement missing"))?;
            prepared
                .decision
                .request()
                .validate(event, ack, now_ms)
                .map_err(|_| Error::new(E_FAIL, "Surface batch decision became stale"))?;
            self.lifecycle
                .take_batch_decision(
                    &receipt.events,
                    self.snapshot_revision,
                    prepared.index,
                    ack,
                    &prepared.decision,
                )
                .ok_or_else(|| Error::new(E_FAIL, "Surface batch decision owner unavailable"))
        })();
        match transfer {
            Ok(owner) => Ok(BatchDecisionInput {
                owner,
                revision: self.snapshot_revision,
                prepared,
            }),
            Err(error) => Err(Box::new(BatchDecisionStartError { prepared, error })),
        }
    }
}
