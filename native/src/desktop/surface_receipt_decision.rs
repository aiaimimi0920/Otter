//! 收据到决定任务的单次所有权交接；失败完整返还，不重新构造事件或丢弃用户选择。
use super::*;
use crate::{surface_confirmation::Decision, surface_decision_runtime::Input};

pub struct ReceiptDecisionError {
    pub receipt: EventReceipt,
    pub decision: Decision,
    pub error: Error,
}
impl AuthorizedWindow {
    /// 仅消费 prepare_decision/prompt_decision 返回的未尝试决定及其原收据。
    pub fn begin_receipt_decision(
        &mut self,
        receipt: EventReceipt,
        decision: Decision,
        now_ms: u64,
    ) -> std::result::Result<Input, Box<ReceiptDecisionError>> {
        let transfer = (|| {
            // 待决定收据不应包含已消费输出或另一份提示/决定。禁止静默丢弃这些证据。
            if receipt.confirmation.is_some()
                || receipt.decision.is_some()
                || receipt.commit.is_some()
                || receipt.delivery.is_err()
                || !matches!(receipt.presentation, Ok(None))
                || receipt
                    .convergence
                    .as_ref()
                    .is_some_and(|value| value.is_ok())
                || decision.attempted()
            {
                return Err(Error::new(
                    E_FAIL,
                    "Surface receipt cannot enter decision task",
                ));
            }
            let ack = receipt
                .result
                .as_ref()
                .map_err(|_| Error::new(E_FAIL, "Missing Surface acknowledgement"))?;
            if !decision.matches(&receipt.event, ack) {
                return Err(Error::new(
                    E_FAIL,
                    "Surface decision does not match receipt",
                ));
            }
            self.begin_decision(&receipt.event, ack, &decision, now_ms)?
                .ok_or_else(|| Error::new(E_FAIL, "Surface decision owner unavailable"))
        })();
        match transfer {
            Ok(owner) => Ok(Input {
                owner,
                event: receipt.event,
                ack: receipt.result.expect("validated acknowledgement"),
                decision,
                revision: self.snapshot_revision,
            }),
            Err(error) => Err(Box::new(ReceiptDecisionError {
                receipt,
                decision,
                error,
            })),
        }
    }
}
