//! 显式接续选择事务；以原批次首事件标识整批，取消/失败不拆散历史证据。
use super::*;
use crate::{
    desktop::surface_authorized_window::BatchReceipt,
    surface_event_batch_runtime::{
        continuation::{Input, Outcome},
        Error,
    },
};
pub enum BatchResumeAction {
    Keep(Box<BatchReceipt>),
    Start(Config, Lease, Box<Input>),
}
impl Sessions {
    pub fn pending_batch_resume_for(
        &self,
        origin: &str,
        mut available: impl FnMut(&BatchReceipt) -> bool,
    ) -> Option<&str> {
        self.batch_receipts.iter().find_map(|receipt| {
            let first = receipt.events.first()?;
            (first.binding().origin() == origin
                && receipt.continuation_boundary().is_some()
                && available(receipt))
            .then_some(first.id())
        })
    }
    pub fn dispatch_batch_resume(
        &mut self,
        batch_id: &str,
        prepare: impl FnOnce(BatchReceipt) -> BatchResumeAction,
    ) -> io::Result<bool> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let Some(index) = self.batch_receipts.iter().position(|receipt| {
            receipt
                .events
                .first()
                .is_some_and(|event| event.id() == batch_id)
                && receipt.continuation_boundary().is_some()
        }) else {
            return Ok(false);
        };
        if self.occupied() >= MAX_SESSIONS {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        // 回调可能已经移交 owner；先保证启动失败也有结果存放位置。
        self.reserve_batch_outcomes()?;
        let receipt = self.batch_receipts.remove(index);
        match prepare(receipt) {
            BatchResumeAction::Keep(receipt) => {
                self.batch_receipts.insert(index, *receipt);
                Ok(false)
            }
            BatchResumeAction::Start(config, lease, input) => {
                match self.start_surface_batch_continuation(config, lease, *input) {
                    Ok(()) => Ok(true),
                    Err(failure) => {
                        let (outcome, _) = Outcome {
                            input: *failure.input,
                            result: Err(Error::Stopped),
                            delivery: Ok(()),
                        }
                        .into_batch();
                        self.batch_outcomes.push(outcome);
                        Err(failure.error)
                    }
                }
            }
        }
    }
}
