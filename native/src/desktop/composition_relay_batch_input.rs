//! Click-batch preparation transaction: check capacity/reserve results before transferring window ownership, and settle even on startup failure.
use super::*;
use crate::surface_event_batch_runtime::{Error, Input, Outcome};
impl Sessions {
    /// New clicks must not change an attachment's scene while unsettled, unpresented, or awaiting output application.
    pub fn surface_input_ready(&self, binding: &crate::surface_live_binding::Binding) -> bool {
        self.surface_input_ready_with(binding, |_| false)
    }
    pub fn surface_input_ready_with(
        &self,
        binding: &crate::surface_live_binding::Binding,
        mut applied: impl FnMut(&crate::surface_result_commit::Commit) -> bool,
    ) -> bool {
        if self.closed
            || self.occupied() >= MAX_SESSIONS
            || self.event_backlog() >= surface_events::MAX_EVENT_RECEIPTS
        {
            return false;
        }
        !self.event_receipts.iter().any(|receipt| {
            receipt.event.binding() == binding
                && (!matches!(receipt.presentation, Ok(Some(_)))
                    || receipt
                        .commit
                        .as_ref()
                        .is_some_and(|commit| !applied(commit)))
        }) && !self.batch_receipts.iter().any(|receipt| {
            receipt
                .events
                .iter()
                .any(|event| event.binding() == binding)
                && (!matches!(receipt.presentation, Ok(Some(_)))
                    || receipt
                        .observation
                        .as_ref()
                        .and_then(|value| value.as_ref().ok())
                        .is_none_or(|observed| {
                            observed.items.iter().any(|item| {
                                item.commit.as_ref().is_some_and(|commit| !applied(commit))
                            })
                        }))
        })
    }
    /// Callbacks may prepare at most limit original events; None is allowed before ownership transfer.
    pub fn dispatch_surface_batch(
        &mut self,
        config: Config,
        lease: Lease,
        prepare: impl FnOnce(usize) -> io::Result<Option<Input>>,
    ) -> io::Result<bool> {
        if self.closed {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let limit = surface_events::MAX_EVENT_RECEIPTS.saturating_sub(self.event_backlog());
        if self.occupied() >= MAX_SESSIONS || limit == 0 {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        self.reserve_batch_outcomes()?;
        let Some(input) = prepare(limit)? else {
            return Ok(false);
        };
        match self.start_surface_batch(config, lease, input) {
            Ok(()) => Ok(true),
            Err(failure) => {
                self.batch_outcomes.push(Outcome {
                    input: *failure.input,
                    submission: Err(Error::NotStarted),
                    observation: None,
                    delivery: Ok(()),
                });
                Err(failure.error)
            }
        }
    }
}
