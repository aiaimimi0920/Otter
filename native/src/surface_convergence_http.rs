//! Deadline-bound single-action convergence reads; GET only, no POST replay, and lease cancellation can wake waits directly.
use super::*;
use crate::{
    surface_action_ack::{Ack, Status},
    surface_click_event::ClickEvent,
    surface_convergence::{Snapshot, Tracker},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConvergenceError {
    Identity,
    Stopped,
    Deadline,
    ConfirmationRequired,
    Stream(StreamError),
    State(crate::surface_convergence::Error),
}
pub struct ConvergenceOutcome {
    pub ack: Ack,
    pub result: Result<Snapshot, ConvergenceError>,
}
impl Control<'_> {
    /// Callers may only shorten the 25-second total budget; success is not UI delivery permission, and consumption must still recheck the lease.
    pub fn converge_surface(
        &self,
        event: &ClickEvent,
        ack: &Ack,
        deadline: Instant,
    ) -> ConvergenceOutcome {
        self.converge_mode(event, ack, deadline, false)
    }
    /// After HTTP confirms a decision, observe only the original action without another POST or automatic approval.
    pub fn converge_decided_surface(
        &self,
        event: &ClickEvent,
        ack: &Ack,
        decision: &crate::surface_confirmation::Decision,
        deadline: Instant,
    ) -> ConvergenceOutcome {
        if !decision.confirmed() || !decision.matches(event, ack) {
            return ConvergenceOutcome {
                ack: ack.clone(),
                result: Err(ConvergenceError::Identity),
            };
        }
        self.converge_mode(event, ack, deadline, true)
    }
    fn converge_mode(
        &self,
        event: &ClickEvent,
        ack: &Ack,
        deadline: Instant,
        decided: bool,
    ) -> ConvergenceOutcome {
        let deadline = deadline.min(Instant::now() + Duration::from_secs(25));
        if event.binding().origin() != self.client.origin()
            || event.binding().device() != self.session.device_id()
        {
            return ConvergenceOutcome {
                ack: ack.clone(),
                result: Err(ConvergenceError::Identity),
            };
        }
        let mut tracker = match Tracker::new(event, ack) {
            Ok(tracker) => tracker,
            Err(error) => {
                return ConvergenceOutcome {
                    ack: ack.clone(),
                    result: Err(ConvergenceError::State(error)),
                }
            }
        };
        let result = self.read_convergence(&mut tracker, deadline, decided);
        ConvergenceOutcome {
            ack: tracker.ack().clone(),
            result,
        }
    }
    fn convergence_gate(&self, deadline: Instant) -> Result<(), ConvergenceError> {
        if self.cancel.is_stopped() {
            return Err(ConvergenceError::Stopped);
        }
        if Instant::now() >= deadline {
            return Err(ConvergenceError::Deadline);
        }
        let now = now().map_err(|error| ConvergenceError::Stream(StreamError::Transport(error)))?;
        self.session
            .token_for(self.client.origin(), now)
            .map_err(|_| ConvergenceError::Stream(StreamError::Transport(Error::Credential)))?;
        Ok(())
    }
    fn read_convergence(
        &self,
        tracker: &mut Tracker<'_>,
        deadline: Instant,
        decided: bool,
    ) -> Result<Snapshot, ConvergenceError> {
        loop {
            self.convergence_gate(deadline)?;
            if !decided && tracker.ack().status() == Status::AwaitingConfirmation {
                return Err(ConvergenceError::ConfirmationRequired);
            }
            let was_success = tracker.ack().succeeded();
            let batch = self.surface_stream_until(tracker.after(), deadline);
            self.convergence_gate(deadline)?;
            let batch = batch.map_err(ConvergenceError::Stream)?;
            tracker.apply(&batch).map_err(ConvergenceError::State)?;
            // The state machine retains only necessary state; release batches of up to 8 MiB before awaiting the next read.
            drop(batch);
            self.convergence_gate(deadline)?;
            if let Some(snapshot) = tracker.take_snapshot() {
                return Ok(snapshot);
            }
            if !decided && tracker.ack().status() == Status::AwaitingConfirmation {
                return Err(ConvergenceError::ConfirmationRequired);
            }
            // Refresh immediately after newly observed success; bound other read waits to prevent empty streams or incomplete refreshes from busy-looping.
            if !was_success && tracker.ack().succeeded() {
                continue;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if !self.cancel.wait(Duration::from_millis(200).min(remaining)) {
                return Err(ConvergenceError::Stopped);
            }
        }
    }
}
