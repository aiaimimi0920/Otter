//! Receiver-owner termination and stabilization: ordinary cancellation does not join, and actual disk inclusion synchronously revokes automatic-compensation eligibility.
use super::*;
use crate::projection_delivery::{binding::checkpoint::Index, journal::Ticket};

impl Saved {
    pub(super) fn stabilize(&mut self) {
        self.held.stabilize();
        self.durable = true;
    }
}
impl Owner {
    pub(in crate::desktop::application) fn stop(&mut self, cleanup: &mut super::cleanup::Owner) {
        self.1 = Default::default();
        match &self.0 {
            Phase::Preparing(pending) => pending.job.stop(), // UI does not join and still exclusively holds the original slot/activity permission.
            Phase::Reporting(reporting) => reporting.job.request_stop(),
            Phase::Recovering(pending) => pending.job.request_stop(),
            Phase::Syncing(pending) => pending.job.request_stop(),
            _ => {
                let phase = std::mem::take(&mut self.0);
                let ticket = match &phase {
                    Phase::Checkpoint(saved) => saved.held.cleanup_ticket(),
                    _ => None,
                };
                drop(phase);
                if let Some(ticket) = ticket {
                    cleanup.abandon(ticket);
                }
            }
        }
    }
    pub(in crate::desktop::application) fn shutdown(
        &mut self,
        cleanup: &mut super::cleanup::Owner,
    ) {
        self.stop(cleanup);
        let phase = std::mem::take(&mut self.0);
        let ticket = match &phase {
            Phase::Preparing(pending) => pending.job.cleanup_ticket(),
            _ => None, // Reporting must already be actually saved; receipt failure is not deletion evidence.
        };
        drop(phase); // Reap synchronously only at terminal state; finish the original worker/activity permission before queuing a marker.
        if let Some(ticket) = ticket {
            cleanup.abandon(ticket);
        }
    }
    pub(in crate::desktop::application) fn committed(
        &mut self,
        index: Option<&Index>,
        workspace: &crate::workspace::Workspace,
    ) {
        if let Phase::Checkpoint(saved) = &mut self.0 {
            if saved.workspace.matches(workspace) {
                match index {
                    Some(index) if index.includes(&saved.expected) => saved.stabilize(),
                    // Old/corrupt associations do not block ordinary saving; unknown indexes prove neither absence of persistence nor eligibility for automatic unlinking.
                    // Fall back only to explicit recovery, without granting durable, presentation, or displayed eligibility.
                    None => saved.held.stabilize(),
                    _ => {}
                }
            }
        }
    }
}
impl Application {
    pub(super) fn receive_cleanup(&mut self, ticket: Option<Ticket>) {
        if let Some(ticket) = ticket {
            self.projection_cleanup.abandon(ticket);
        }
    }
}
