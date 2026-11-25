//! Sequential background dispatch: readmit after each connection and advance only on success; no automatic retries or remote rollback.
use super::{exchange, Batch, Request};
use crate::{byte_budget::Budget, loom_bridge_client, loom_bridge_identity::Identity};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Debug, PartialEq)]
pub enum Failure {
    Prepare(super::Error),
    Connect(loom_bridge_client::Error),
    NotAdmitted,
    Cancelled,
    Exchange(exchange::Error),
}
#[derive(Debug, PartialEq)]
pub struct Report {
    /// Count only strictly matching successful receipts received by this call, excluding messages confirmed by previous calls.
    pub acknowledged: usize,
    pub failure: Option<Failure>,
    /// Whether the current message may have reached the remote endpoint; false still cannot disprove commit by a prior attempt.
    pub may_have_reached_remote: bool,
}
/// Must run in the background. admit cannot directly access HWND/Rc state; the owner provides a thread-safe
/// gate or UI handshake. Returning true authorizes only the current request, not the entire batch permanently.
/// Failure preserves the batch's current request and ID; callers must not invoke again for automatic retry without fresh validation.
pub fn run(
    batch: &mut Batch<'_>,
    identity: &Identity,
    budget: &Arc<Budget>,
    limit: usize,
    cancel: &AtomicBool,
    mut admit: impl FnMut(&Request) -> bool,
) -> Report {
    let mut report = Report {
        acknowledged: 0,
        failure: None,
        may_have_reached_remote: false,
    };
    loop {
        if cancel.load(Ordering::Acquire) {
            report.failure = Some(Failure::Cancelled);
            return report;
        }
        let request = match batch.current(budget, limit, cancel) {
            Ok(Some(request)) => request,
            Ok(None) => return report,
            Err(error) => {
                report.failure = Some(Failure::Prepare(error));
                return report;
            }
        };
        let socket = match loom_bridge_client::connect(identity, Duration::from_secs(5)) {
            Ok(socket) => socket,
            Err(error) => {
                report.failure = Some(Failure::Connect(error));
                return report;
            }
        };
        // Connecting may take time; only a fresh post-connection gate authorizes sending message content.
        if cancel.load(Ordering::Acquire) {
            report.failure = Some(Failure::Cancelled);
            return report;
        }
        if !admit(request) {
            report.failure = Some(Failure::NotAdmitted);
            return report;
        }
        match exchange::run(socket, request, budget, cancel) {
            Ok(()) => {
                batch.consume_pending();
                report.acknowledged += 1;
                if batch.remaining() == 0 {
                    return report;
                }
            }
            Err(error) => {
                // Cancelled may occur before sending or while waiting for receipts; it does not prove absence of commit.
                report.may_have_reached_remote = !matches!(error, exchange::Error::Budget);
                report.failure = Some(Failure::Exchange(error));
                return report;
            }
        }
    }
}
