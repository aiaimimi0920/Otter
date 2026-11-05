//! 后台恢复协作权威/可见标注；先校验同一canonical摘要与工作区身份，不从标注猜原wire。
use super::projection_edit_workspace::Owner;
use crate::{projection_edit::workspace::Snapshot, workspace_id::UnitId};
use std::sync::atomic::{AtomicBool, Ordering};
use windows::{
    core::{Error, Result},
    Win32::Foundation::{E_ABORT, E_INVALIDARG},
};

pub(super) fn prepare(
    document: &crate::session_document::Document<'_>,
    vectors: &mut [(UnitId, crate::session_vectors::Restored)],
    cancel: &AtomicBool,
) -> Result<Vec<(UnitId, Owner)>> {
    if document
        .windows
        .iter()
        .filter(|window| window.otter_native_projection_edit.is_some())
        .count()
        > crate::projection_edit::workspace::MAX_ENTRIES
    {
        return Err(Error::new(
            E_INVALIDARG,
            "Projection edit workspace capacity exceeded",
        ));
    }
    let mut result = Vec::new();
    for window in &document.windows {
        if cancel.load(Ordering::Acquire) {
            return Err(Error::new(E_ABORT, "Projection edit restoration cancelled"));
        }
        let snapshot = Snapshot::restore(window, document.projection_scope).map_err(|error| {
            Error::new(
                E_INVALIDARG,
                format!("Projection edit workspace restore rejected: {error:?}"),
            )
        })?;
        if let Some(snapshot) = snapshot {
            let (unit, restored) = vectors
                .iter_mut()
                .find(|(unit, _)| unit.as_str() == window.id)
                .ok_or_else(|| {
                    Error::new(E_INVALIDARG, "Missing projection edit restoration target")
                })?;
            let (editor, owner) = Owner::restore(snapshot, std::mem::take(&mut restored.editor))?;
            restored.ids = editor
                .elements()
                .iter()
                .map(|s| s.annotation_id().to_owned())
                .collect();
            restored.editor = editor;
            result.push((unit.clone(), owner));
        }
    }
    Ok(result)
}
