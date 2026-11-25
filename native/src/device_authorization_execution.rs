//! Separate execution stop from authentication revocation; leave bounded cleanup opportunity only for the same serial owner.
use super::*;

/// The stopper holds only cancellation capability, without borrowing or transferring original credentials.
pub(crate) struct ExecutionStop(Arc<State>);
impl ExecutionStop {
    pub(crate) fn stop(&self) {
        self.0.stop_execution();
    }
}

impl State {
    pub(super) fn stop_execution(&self) {
        self.execution_cancel.cancel();
        self.execution_watch.revoke();
    }
}

impl Lease {
    /// Host stop immediately interrupts execution-network/child waits; it neither revokes authentication nor permits execution resumption.
    pub(crate) fn stop_execution(&self) {
        self.state.stop_execution();
    }
    pub(crate) fn execution_stop_handle(&self) -> ExecutionStop {
        ExecutionStop(self.state.clone())
    }
    pub(crate) fn execution_stopped(&self) -> bool {
        self.is_revoked() || self.state.execution_cancel.is_stopped()
    }
    pub(crate) fn execution_notification(&self) -> std::io::Result<Notification> {
        self.state.execution_watch.notification()
    }
    /// Execution/cleanup remain serial without new credentials or leases; authorization revocation cancels both network signals.
    pub(crate) fn execution_control<'a>(
        &'a self,
        client: &'a Client,
    ) -> Result<Control<'a>, Error> {
        if self.execution_stopped() {
            return Err(Error::Revoked);
        }
        Ok(Control::new(
            client,
            self.online_session()?,
            &self.state.execution_cancel,
        ))
    }
}
