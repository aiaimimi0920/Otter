//! 只借出已呈现的正式结果；原始 Commit 留在收据中用于恢复，不赋予资源访问权。
use super::*;
use crate::{surface_live_binding::Binding, surface_result_commit::Commit};
impl Sessions {
    pub fn visit_surface_outputs(&self, mut apply: impl FnMut(&Binding, &Commit, u64)) {
        for receipt in &self.event_receipts {
            let Ok(Some(revision)) = receipt.presentation else {
                continue;
            };
            let Ok(ack) = &receipt.result else { continue };
            if receipt.delivery.is_err() || !ack.matches_event(&receipt.event) || !ack.succeeded() {
                continue;
            }
            if let Some(commit) = &receipt.commit {
                apply(receipt.event.binding(), commit, revision);
            }
        }
        for receipt in &self.batch_receipts {
            let Ok(Some(revision)) = receipt.presentation else {
                continue;
            };
            if receipt.delivery.is_err() {
                continue;
            }
            let Ok(submitted) = &receipt.submission else {
                continue;
            };
            let Some(Ok(observed)) = &receipt.observation else {
                continue;
            };
            if submitted.stop != crate::live_relay_http::BatchStop::Complete
                || submitted.acknowledgements.len() != receipt.events.len()
                || observed.items.len() != receipt.events.len()
            {
                continue;
            }
            if !receipt
                .events
                .iter()
                .zip(&submitted.acknowledgements)
                .zip(&observed.items)
                .all(|((event, ack), item)| {
                    ack.matches_event(event) && ack.succeeded() && item.result.is_ok()
                })
            {
                continue;
            }
            for (event, item) in receipt.events.iter().zip(&observed.items) {
                if let Some(commit) = &item.commit {
                    apply(event.binding(), commit, revision);
                }
            }
        }
    }
}
