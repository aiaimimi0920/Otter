//! Bounded compensation after background execution ends; no retry queues/threads or disguising failed leases as success.
use super::{Completion, Failure};
use crate::loom_art_control::{self, Outcome};
use std::{thread, time::Duration};
impl Completion {
    fn cancellation_pending(&self) -> bool {
        matches!(
            &self.remote_cancel,
            Some(Err(_)) | Some(Ok(Outcome::Failed | Outcome::Cancelled))
        )
    }
    pub fn cleanup_pending(&self) -> bool {
        self.cancellation_pending()
            || matches!(&self.result,Err(Failure::PendingRelease(delivery)) if delivery.pending_release_count()>0)
    }
    /// At most two compensations after initial failure, waiting on original execution threads; Completion retains unfinished results.
    pub(super) fn retry_cleanup(&mut self) {
        for delay in [Duration::from_millis(100), Duration::from_millis(500)] {
            if !self.cleanup_pending() {
                break;
            }
            thread::sleep(delay);
            self.cleanup_retries += 1;
            if self.cancellation_pending() {
                self.remote_cancel = Some(loom_art_control::cancel_authenticated(
                    &self.identity,
                    &self.unit,
                    &self.request,
                ));
            }
            if let Err(Failure::PendingRelease(delivery)) = &mut self.result {
                if delivery.pending_release_count() > 0 {
                    let _ = delivery.release_authenticated(&self.identity);
                }
            }
        }
    }
}
