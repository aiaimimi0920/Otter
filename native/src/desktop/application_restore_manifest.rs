//! Validate one-to-one restore-batch correspondence before window creation; missing/duplicate nodes must not be partially published.
use super::*;

pub(super) fn validate(prepared: &Prepared) -> Result<()> {
    let candidate = &prepared.candidate;
    let units = candidate.workspace.units();
    let count = units.len();
    if count > crate::workspace::MAX_UNITS
        || prepared.states.len() != count
        || candidate.placements.len() != count
        || candidate.images.len() != count
        || prepared.annotation_ids.len() != count
    {
        return Err(invalid());
    }
    // prepare_states builds a dense batch in workspace order; revalidate before creating windows, including same-count duplicates or reordered entries.
    for ((((unit, state), placement), images), (annotation_unit, ids)) in units
        .iter()
        .zip(&prepared.states)
        .zip(&candidate.placements)
        .zip(&candidate.images)
        .zip(&prepared.annotation_ids)
    {
        if state.unit_id != *unit
            || placement.unit != *unit
            || images.unit != *unit
            || annotation_unit != unit
        {
            return Err(invalid());
        }
        let group = candidate.workspace.group_of(unit);
        let canvas = state.canvas.borrow();
        if ids.len() > 4096
            || ids.len() != canvas.editor.elements().len()
            || !ids.iter().map(String::as_str).eq(canvas
                .editor
                .elements()
                .iter()
                .map(|stroke| stroke.annotation_id()))
            || !canvas.pinned
            || state.global_capture.get()
            || state.global_long_capture.get()
            || state.global_live_capture.get()
            || state.restored_art != placement.art
            || state.execution_results.borrow().unit() != unit
            || state.group_id.borrow().as_ref() != group.map(|group| &group.id)
            || state.group_locked.get() != group.is_some_and(|group| group.locked)
            || state.group_hidden.get() != group.is_some_and(|group| group.hidden)
        {
            return Err(invalid());
        }
        let mut seen = std::collections::HashSet::with_capacity(ids.len());
        if ids.iter().any(|id| {
            id.is_empty()
                || id.len() > 256
                || id.chars().any(char::is_control)
                || !seen.insert(id.as_str())
        }) {
            return Err(invalid());
        }
    }
    Ok(())
}

fn invalid() -> Error {
    Error::new(E_INVALIDARG, "Restored workspace manifest is inconsistent")
}
