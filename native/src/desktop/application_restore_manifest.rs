//! 建窗前验证恢复批次的一一对应关系；不允许缺失/重复节点被部分发布。
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
    // prepare_states 按工作区顺序创建密集批次；在建窗前重新验证，包含同数量的重复或错序。
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
