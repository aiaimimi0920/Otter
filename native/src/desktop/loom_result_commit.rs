//! Synchronous transaction boundary from completion reports to node outputs; no message loops/network or discarded cleanup owners.
use super::{Completion, Report};
use crate::{
    desktop::{loom_delivery::TakeError, loom_result_state::Results},
    editor::Editor,
    graph_requests::Requests,
    workspace_id::UnitId,
};

#[derive(Debug, PartialEq)]
pub enum CommitError {
    Stale,
    ExecutionFailed,
    PendingCleanup,
    Output(TakeError),
    State(crate::desktop::loom_result_state::Error),
}
impl Report {
    /// Cleanup after UI rejection; end only still-matching requests, while completion retains resources awaiting cleanup.
    /// Old documents need not remain valid: document changes themselves may cause commit rejection.
    pub fn reject(&mut self, registry: &mut Requests, state: &mut Results, message: &str) -> bool {
        if state.unit() != &self.unit
            || !registry.is_latest(&self.unit, &self.request)
            || state.active() != Some(&self.request)
        {
            return false;
        }
        if !state.fail(&self.request, message) {
            return false;
        }
        let finished = registry.finish(&self.unit, &self.request);
        debug_assert!(finished);
        self.current = false;
        true
    }
    /// Transfer candidate metadata after successful official-port commit; later consumers own candidate selection/cache merging.
    /// lookup must be read-only and nonreentrant into UI; afterward this report cannot commit another node.
    pub fn commit<'a>(
        &mut self,
        registry: &mut Requests,
        state: &mut Results,
        lookup: impl FnMut(&UnitId) -> Option<&'a Editor>,
    ) -> Result<(), CommitError> {
        let mut lookup = lookup;
        self.commit_checked(registry, state, |unit, stamp| {
            lookup(unit).is_some_and(|editor| stamp.matches(editor) && editor.draft().is_none())
        })
    }
    pub(crate) fn commit_checked(
        &mut self,
        registry: &mut Requests,
        state: &mut Results,
        check: impl FnMut(&UnitId, &crate::desktop::loom_input_stamp::SourceStamp) -> bool,
    ) -> Result<(), CommitError> {
        if state.unit() != &self.unit
            || state.active() != Some(&self.request)
            || !self.current
            || !registry.is_latest(&self.unit, &self.request)
            || !self.freshness.is_current(check)
        {
            return Err(CommitError::Stale);
        }
        match &mut self.completion {
            Completion::Shader(completion) => {
                let completion = completion
                    .as_mut()
                    .map_err(|_| CommitError::ExecutionFailed)?;
                let batch = completion
                    .take(&self.unit, &self.request)
                    .ok_or(CommitError::Stale)?;
                state
                    .finish(&self.request, batch)
                    .map_err(CommitError::State)?;
            }
            Completion::Remote(completion) => {
                let completion = completion
                    .as_mut()
                    .map_err(|_| CommitError::ExecutionFailed)?;
                if completion.cleanup_pending() {
                    return Err(CommitError::PendingCleanup);
                }
                let result = completion
                    .result
                    .as_mut()
                    .map_err(|_| CommitError::ExecutionFailed)?;
                // Independently compare report/underlying delivery identity, preventing incorrectly assembled public reports from committing across nodes.
                if completion.unit != self.unit
                    || completion.request != self.request
                    || result.delivery.unit() != &self.unit
                    || result.delivery.request() != &self.request
                {
                    return Err(CommitError::Stale);
                }
                let batch = result
                    .delivery
                    .take_released()
                    .map_err(CommitError::Output)?;
                state
                    .finish(&self.request, batch)
                    .map_err(CommitError::State)?;
                state.receive_candidates(result.candidates.take());
            }
        }
        // No external callbacks/await occur above, leaving no scheduling gap for registry replacement after validation.
        let finished = registry.finish(&self.unit, &self.request);
        debug_assert!(finished);
        self.current = false;
        Ok(())
    }
}
