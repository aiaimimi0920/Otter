//! 批次确认选择事务：取消完整返还，启动失败保留决定，绝不拆散整批证据。
use super::*;
use crate::{
    desktop::surface_authorized_window::BatchReceipt,
    surface_batch_decision_runtime::{Error, Input, Outcome},
};
pub enum BatchConfirmationAction {
    Keep(Box<BatchReceipt>),
    Start(Config, Lease, Box<Input>),
}
impl Sessions {
    pub fn pending_batch_confirmation_for(
        &self,
        origin: &str,
        now_ms: u64,
        mut available: impl FnMut(&BatchReceipt, usize) -> bool,
    ) -> Option<&str> {
        for receipt in &self.batch_receipts {
            for (index, event) in receipt.events.iter().enumerate() {
                if event.binding().origin() == origin
                    && receipt.confirmation_ready(index, now_ms)
                    && available(receipt, index)
                {
                    return Some(event.id());
                }
            }
        }
        None
    }
    pub fn dispatch_batch_confirmation(
        &mut self,
        event_id: &str,
        prepare: impl FnOnce(BatchReceipt, usize) -> BatchConfirmationAction,
    ) -> io::Result<bool> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_millis();
        let now = u64::try_from(now).map_err(io::Error::other)?;
        let selected = self
            .batch_receipts
            .iter()
            .enumerate()
            .find_map(|(batch, receipt)| {
                receipt
                    .events
                    .iter()
                    .enumerate()
                    .find_map(|(index, event)| {
                        (event.id() == event_id && receipt.confirmation_ready(index, now))
                            .then_some((batch, index))
                    })
            });
        let Some((batch, index)) = selected else {
            return Ok(false);
        };
        if self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        // 在提示消费及 owner 移交之前，预留成功/异常两种返还位置。
        self.reserve_batch_outcomes()?;
        self.batch_decision_rejected
            .try_reserve(self.batch_decision_jobs.len() + 1)
            .map_err(io::Error::other)?;
        let receipt = self.batch_receipts.remove(batch);
        match prepare(receipt, index) {
            BatchConfirmationAction::Keep(receipt) => {
                self.batch_receipts.insert(batch, *receipt);
                Ok(false)
            }
            BatchConfirmationAction::Start(config, lease, input) => {
                match self.start_surface_batch_decision(config, lease, *input) {
                    Ok(()) => Ok(true),
                    Err(failure) => {
                        let outcome = Outcome {
                            input: *failure.input,
                            submission: Err(Error::NotStarted),
                            convergence: None,
                            delivery: Ok(()),
                        };
                        match outcome.into_batch() {
                            Ok(outcome) => self.batch_outcomes.push(outcome),
                            Err(outcome) => self.batch_decision_rejected.push(*outcome),
                        }
                        Err(failure.error)
                    }
                }
            }
        }
    }
}
