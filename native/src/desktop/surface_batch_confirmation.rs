//! 批次明确选择后的提示消费；整批收据始终与决定一起持有，不拆散其他事件证据。
use super::*;
use crate::surface_confirmation::{Choice, Decision, Request};
pub struct PreparedBatchDecision {
    pub receipt: BatchReceipt,
    pub index: usize,
    pub decision: Decision,
}
pub struct BatchDecisionError {
    pub receipt: BatchReceipt,
    pub error: Error,
}
impl BatchReceipt {
    /// 只审核收据证据；真正显示提示前仍须通过窗口授权和当前版本检查。
    pub fn confirmation_ready(&self, index: usize, now_ms: u64) -> bool {
        use crate::surface_event_batch_runtime::ObservationError;
        if self.delivery.is_err()
            || !matches!(
                self.presentation,
                Ok(None) | Err(PresentationError::Evidence)
            )
        {
            return false;
        }
        let Some(event) = self.events.get(index) else {
            return false;
        };
        let Ok(submitted) = &self.submission else {
            return false;
        };
        let Some(Ok(observed)) = &self.observation else {
            return false;
        };
        if submitted.acknowledgements.len() > self.events.len()
            || observed.items.len() != submitted.acknowledgements.len()
            || event.revision() != self.revision
            || !event.attempted()
        {
            return false;
        }
        let Some(ack) = submitted.acknowledgements.get(index) else {
            return false;
        };
        let Some(item) = observed.items.get(index) else {
            return false;
        };
        item.decision.is_none()
            && item.commit.is_none()
            && item.result
                == Err(ObservationError::Convergence(
                    crate::live_relay_http::ConvergenceError::ConfirmationRequired,
                ))
            && item
                .confirmation
                .as_ref()
                .and_then(|value| value.as_ref().ok())
                .is_some_and(|request| request.validate(event, ack, now_ms).is_ok())
    }
}
impl AuthorizedWindow {
    pub fn batch_confirmation_for<'a>(
        &mut self,
        receipt: &'a BatchReceipt,
        index: usize,
        now_ms: u64,
    ) -> Result<&'a Request> {
        self.refresh(now_ms)?;
        if !receipt.confirmation_ready(index, now_ms) {
            return Err(Error::new(
                E_FAIL,
                "Surface batch confirmation evidence unavailable",
            ));
        }
        // 部分完成时窗口会以 Evidence 拒绝整批呈现，但有效待确认项仍可让用户选择。
        if self.stopped.is_some()
            || !self.is_shown()
            || receipt.delivery.is_err()
            || receipt.revision != self.snapshot_revision
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
            || !matches!(
                receipt.presentation,
                Ok(None) | Err(PresentationError::Evidence)
            )
        {
            return Err(Error::new(
                E_FAIL,
                "Surface batch confirmation is not authorized",
            ));
        }
        let event = receipt
            .events
            .get(index)
            .ok_or_else(|| Error::new(E_FAIL, "Surface batch event missing"))?;
        if event.binding() != self.binding() || event.revision() != self.snapshot_revision {
            return Err(Error::new(E_FAIL, "Surface batch event became stale"));
        }
        let observed = receipt
            .observation
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .ok_or_else(|| Error::new(E_FAIL, "Surface batch observation missing"))?;
        let item = observed
            .items
            .get(index)
            .ok_or_else(|| Error::new(E_FAIL, "Surface batch item missing"))?;
        let request = item
            .confirmation
            .as_ref()
            .and_then(|value| value.as_ref().ok())
            .ok_or_else(|| Error::new(E_FAIL, "Surface batch confirmation missing"))?;
        Ok(request)
    }
    /// 调用方须在明确选择之后调用；失败完整返还，成功不发网络请求、不移交 owner。
    pub fn prepare_batch_decision(
        &mut self,
        mut receipt: BatchReceipt,
        index: usize,
        choice: Choice,
        now_ms: u64,
    ) -> std::result::Result<PreparedBatchDecision, Box<BatchDecisionError>> {
        if let Err(error) = self.batch_confirmation_for(&receipt, index, now_ms) {
            return Err(Box::new(BatchDecisionError { receipt, error }));
        }
        let observed = receipt
            .observation
            .as_mut()
            .expect("validated observation")
            .as_mut()
            .expect("validated observation result");
        let request = observed.items[index]
            .confirmation
            .take()
            .expect("validated prompt")
            .expect("validated prompt result");
        let ack = &receipt
            .submission
            .as_ref()
            .expect("validated submission")
            .acknowledgements[index];
        match Decision::new(request, &receipt.events[index], ack, choice, now_ms) {
            Ok(decision) => Ok(PreparedBatchDecision {
                receipt,
                index,
                decision,
            }),
            Err(failure) => {
                observed.items[index].confirmation = Some(Ok(failure.request));
                Err(Box::new(BatchDecisionError {
                    receipt,
                    error: Error::new(E_FAIL, "Surface batch decision preparation failed"),
                }))
            }
        }
    }
}
