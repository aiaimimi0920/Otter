//! 显式接续原批次：已完成前缀不重发，未尝试后缀与原证据始终共同持有。
use super::*;
use crate::{live_relay_http::BatchStop, surface_confirmation::Choice};
pub struct BatchContinuationInput {
    pub owner: Owner,
    pub revision: u64,
    pub receipt: BatchReceipt,
    pub next: usize,
}
impl BatchReceipt {
    /// 返回唯一可接续边界；不是发送许可，窗口必须再次核对当前授权和版本。
    pub fn continuation_boundary(&self) -> Option<usize> {
        if self.delivery.is_err()
            || !matches!(
                self.presentation,
                Ok(None) | Err(PresentationError::Evidence)
            )
        {
            return None;
        }
        let submitted = self.submission.as_ref().ok()?;
        if submitted.stop != BatchStop::Confirmation {
            return None;
        }
        let observed = self.observation.as_ref()?.as_ref().ok()?;
        let next = submitted.acknowledgements.len();
        if next == 0 || next >= self.events.len() || observed.items.len() != next {
            return None;
        }
        let binding = self.events.first()?.binding();
        if self.events.iter().enumerate().any(|(index, event)| {
            event.binding() != binding
                || event.revision() != self.revision
                || event.attempted() != (index < next)
                || self.events[..index]
                    .iter()
                    .any(|prior| prior.id() == event.id())
        }) {
            return None;
        }
        for ((event, ack), item) in self
            .events
            .iter()
            .zip(&submitted.acknowledgements)
            .zip(&observed.items)
        {
            if !ack.matches_event(event)
                || !ack.succeeded()
                || item.confirmation.is_some()
                || !item.result.is_ok_and(|revision| revision >= self.revision)
            {
                return None;
            }
            if let Some(evidence) = &item.decision {
                if evidence.submission.is_err()
                    || !evidence.decision.confirmed()
                    || evidence.decision.choice() != Choice::Approve
                    || evidence.revision != self.revision
                    || !evidence.decision.matches(event, ack)
                {
                    return None;
                }
            }
        }
        // stop=Confirmation 必须有对应的已确认批准，不能仅凭状态字段跨过提示。
        observed.items.last()?.decision.as_ref()?;
        Some(next)
    }
}
impl AuthorizedWindow {
    pub fn batch_continuation_for(&mut self, receipt: &BatchReceipt, now_ms: u64) -> Result<usize> {
        self.refresh(now_ms)?;
        let next = receipt
            .continuation_boundary()
            .ok_or_else(|| Error::new(E_FAIL, "Surface batch has no safe continuation"))?;
        if self.stopped.is_some()
            || !self.is_shown()
            || receipt.revision != self.snapshot_revision
            || receipt.events[0].binding() != self.binding()
            || !self.lifecycle.owner().is_some_and(Owner::confirmed_active)
        {
            return Err(Error::new(
                E_FAIL,
                "Surface batch continuation became stale",
            ));
        }
        Ok(next)
    }
    pub fn begin_batch_continuation(
        &mut self,
        receipt: BatchReceipt,
        now_ms: u64,
    ) -> std::result::Result<BatchContinuationInput, Box<BatchDecisionError>> {
        let checked = (|| {
            let next = self.batch_continuation_for(&receipt, now_ms)?;
            let owner = self
                .lifecycle
                .take_batch_continuation(&receipt.events, self.snapshot_revision, next)
                .ok_or_else(|| Error::new(E_FAIL, "Surface batch owner unavailable"))?;
            Ok((next, owner))
        })();
        match checked {
            Ok((next, owner)) => Ok(BatchContinuationInput {
                owner,
                revision: self.snapshot_revision,
                receipt,
                next,
            }),
            Err(error) => Err(Box::new(BatchDecisionError { receipt, error })),
        }
    }
}
