//! The application owns registered-request configuration evidence; end only matching generations, with remote cancellation/lease release owned by task collections.
use super::{configuration_stamp::Stamp, State};
use crate::graph_requests::{Request, Requests};
pub(super) struct Binding {
    pub request: Request,
    pub stamp: Stamp,
    pub topology: crate::workspace::TopologyStamp,
}
pub(super) fn retire_changed(
    state: &State,
    registry: &mut Requests,
    workspace: &crate::workspace::Workspace,
) {
    let stale = state
        .execution_configuration
        .borrow()
        .as_ref()
        .is_some_and(|binding| {
            !binding.stamp.matches(state) || !binding.topology.matches(workspace)
        });
    if !stale {
        return;
    }
    let binding = state
        .execution_configuration
        .borrow_mut()
        .take()
        .expect("checked binding");
    // Old bindings must neither end replaced registered requests nor overwrite their state.
    if !registry.finish(&state.unit_id, &binding.request) {
        return;
    }
    if state
        .pending_execution
        .borrow()
        .as_ref()
        .is_some_and(|pending| pending.request == binding.request)
    {
        state.pending_execution.borrow_mut().take();
    }
    state
        .execution_results
        .borrow_mut()
        .cancel(&binding.request);
}
pub(super) fn matches(
    state: &State,
    request: &Request,
    workspace: &crate::workspace::Workspace,
) -> bool {
    match state.execution_configuration.borrow().as_ref() {
        Some(binding) => {
            binding.request == *request
                && binding.stamp.matches(state)
                && binding.topology.matches(workspace)
        }
        // Independent graph/model entry points may lack UI selection; documents with selected capabilities must not bypass evidence.
        None => state.selected_art.borrow().is_none(),
    }
}
pub(super) fn clear(state: &State, request: &Request) {
    let matched = state
        .execution_configuration
        .borrow()
        .as_ref()
        .is_some_and(|binding| binding.request == *request);
    if matched {
        state.execution_configuration.borrow_mut().take();
    }
}
#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-loom")))]
#[path = "execution_configuration_tests.rs"]
mod tests;
