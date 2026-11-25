//! Configuration evidence for asynchronous preparation: document identity, current capability-selection identity, and parameter revision, without mutable-state borrows.
use super::State;
use crate::workspace_id::UnitId;
#[derive(Clone, PartialEq)]
pub(crate) struct Stamp {
    unit: UnitId,
    selection: Option<(UnitId, u64, crate::graph_execution_config::Config)>,
    accept_upstream: bool,
    notify_downstream: bool,
    sticker: super::sticker_input::Stamp,
    local: Option<(
        String,
        Option<std::sync::Arc<crate::art_execution_grant::package::ports::Record>>,
    )>,
}
impl Stamp {
    pub(crate) fn capture(state: &State) -> Self {
        Self {
            unit: state.unit_id.clone(),
            accept_upstream: state.accept_upstream.get(),
            notify_downstream: state.node_settings.get().notify_downstream,
            sticker: state.sticker_input.snapshot().stamp(),
            local: state
                .local_art_id
                .as_ref()
                .map(|c| (c.to_string(), c.ports.borrow().clone())),
            selection: state.selected_art.borrow().as_ref().map(|selected| {
                (
                    selected.identity.clone(),
                    selected.parameters.revision(),
                    selected.execution,
                )
            }),
        }
    }
    pub(crate) fn matches(&self, state: &State) -> bool {
        if self.unit != state.unit_id
            || self.accept_upstream != state.accept_upstream.get()
            || self.notify_downstream != state.node_settings.get().notify_downstream
            || self.sticker != state.sticker_input.snapshot().stamp()
        {
            return false;
        }
        match (&self.local, &state.local_art_id) {
            (None, None) => (),
            (Some((id, ports)), Some(config)) if id == &config.to_string() => {
                let current = config.ports.borrow();
                if !match (ports, current.as_ref()) {
                    (None, None) => true,
                    (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
                    _ => false,
                } {
                    return false;
                }
            }
            _ => return false,
        }
        let selected = state.selected_art.borrow();
        match (&self.selection, selected.as_ref()) {
            (None, None) => true,
            (Some((identity, revision, execution)), Some(selected)) => {
                *identity == selected.identity
                    && *revision == selected.parameters.revision()
                    && *execution == selected.execution
            }
            _ => false,
        }
    }
}
/// Discard only stale configurations; application input-task collections still centrally own cancellation/thread reclamation.
pub(super) fn discard_changed(state: &State, workspace: &crate::workspace::Workspace) {
    let stale = state
        .pending_preparation
        .borrow()
        .as_ref()
        .is_some_and(|pending| {
            !pending.selection.matches(state) || !pending.topology.matches(workspace)
        });
    if stale {
        state.pending_preparation.borrow_mut().take();
        *state.preparation_error.borrow_mut() =
            Some(crate::ui_text::text("action-configuration-changed").into());
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-canvas")))]
#[path = "configuration_stamp_tests.rs"]
mod tests;
