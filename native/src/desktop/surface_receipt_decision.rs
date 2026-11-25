//! One-time ownership handoff from receipt to decision task; failure returns everything without reconstructing events or discarding user choices.
use super::*;
use crate::{surface_confirmation::Decision, surface_decision_runtime::Input};

pub struct ReceiptDecisionError {
    pub receipt: EventReceipt,
    pub decision: Decision,
    pub error: Error,
}
impl AuthorizedWindow {
    /// Consume only unattempted decisions returned by prepare_decision/prompt_decision and their original receipts.
    pub fn begin_receipt_decision(
        &mut self,
        receipt: EventReceipt,
        decision: Decision,
        now_ms: u64,
    ) -> std::result::Result<Input, Box<ReceiptDecisionError>> {
        let transfer = (|| {
            // Pending-decision receipts must contain neither consumed outputs nor another prompt/decision; silently discarding that evidence is forbidden.
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
