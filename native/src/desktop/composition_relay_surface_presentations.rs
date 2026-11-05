//! 异步 JS 呈现完成后才释放原收据/输出门；不改写 ACK/动作结果，也不重放提交。
use super::*;
use crate::{desktop::surface_authorized_window::PresentationError, surface_live_binding::Binding};
impl Sessions {
    #[cfg(test)]
    pub(crate) fn surface_event_backlog_for_test(&self) -> usize {
        self.event_backlog()
    }
    #[cfg(test)]
    pub(crate) fn surface_batch_receipt_for_test(
        &self,
    ) -> Option<&crate::desktop::surface_authorized_window::BatchReceipt> {
        self.batch_receipts.last()
    }
    pub(crate) fn settle_surface_presentations(
        &mut self,
        mut finish: impl FnMut(&Binding, u64) -> Result<Option<u64>, PresentationError>,
    ) {
        for receipt in &mut self.event_receipts {
            if let Err(PresentationError::Pending(revision)) = receipt.presentation {
                receipt.presentation = finish(receipt.event.binding(), revision);
            }
        }
        for receipt in &mut self.batch_receipts {
            if let Err(PresentationError::Pending(revision)) = receipt.presentation {
                receipt.presentation = receipt
                    .events
                    .first()
                    .map_or(Err(PresentationError::Evidence), |event| {
                        finish(event.binding(), revision)
                    });
            }
        }
    }
}
