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
    // 节点数受 MAX_UNITS 限制；逐项检查避免额外复制 ID/图片，也拒绝相同数量下的重复替换。
    for unit in units {
        if prepared
            .states
            .iter()
            .filter(|state| state.unit_id == *unit)
            .count()
            != 1
            || candidate
                .placements
                .iter()
                .filter(|item| item.unit == *unit)
                .count()
                != 1
            || candidate
                .images
                .iter()
                .filter(|item| item.unit == *unit)
                .count()
                != 1
            || prepared
                .annotation_ids
                .iter()
                .filter(|(id, _)| id == unit)
                .count()
                != 1
        {
            return Err(invalid());
        }
        let state = prepared
            .states
            .iter()
            .find(|state| state.unit_id == *unit)
            .unwrap();
        let (_, ids) = prepared
            .annotation_ids
            .iter()
            .find(|(id, _)| id == unit)
            .unwrap();
        let group = candidate.workspace.group_of(unit);
        let placement = candidate
            .placements
            .iter()
            .find(|placement| placement.unit == *unit)
            .unwrap();
        if ids.len() > 4096
            || ids.len() != state.canvas.borrow().editor.elements().len()
            || !ids.iter().map(String::as_str).eq(state
                .canvas
                .borrow()
                .editor
                .elements()
                .iter()
                .map(|stroke| stroke.annotation_id()))
            || !state.canvas.borrow().pinned
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
