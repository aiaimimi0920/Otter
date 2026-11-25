//! Session-source candidates before frozen-entry append publication; construct memory only, leaving successful commit to the window owner.
use super::{candidate, Kind, Library};
use crate::{session_append, session_document, workspace_id::UnitId};
use std::sync::atomic::{AtomicBool, Ordering};

/// active is the current workspace identity set; remove source records for closed old windows so they do not consume window budget.
/// Prune only source links invalidated by old windows; collect current topology/geometry together at the next committed save.
pub fn prepare(
    original: &[u8],
    instance: &[u8],
    active: &[UnitId],
    kind: Kind,
    entry_id: &str,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    let source =
        session_document::decode(original).map_err(|error| format!("Invalid source: {error:?}"))?;
    let copy = session_document::decode(instance)
        .map_err(|error| format!("Invalid instance: {error:?}"))?;
    let library = source.library(kind)?;
    prepare_decoded(&source, &copy, active, kind, &library, entry_id, cancel)
}

/// Frozen workers reuse the session and validated library borrowed during instantiation, without rescanning the entire source.
pub(crate) fn prepare_decoded(
    source: &session_document::Document<'_>,
    copy: &session_document::Document<'_>,
    active: &[UnitId],
    kind: Kind,
    library: &Library<'_>,
    entry_id: &str,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Frozen publication cancelled".into());
    }
    let entry = library
        .find(entry_id)
        .ok_or("Frozen entry no longer exists")?;
    let append = session_append::Prepared::new(source, copy, active)?;
    if append.unit_id() == entry.source_sticker_id {
        return Err("Frozen publication identity or capacity conflict".into());
    }
    if !matches_snapshot(entry.snapshot_json(), append.sticker_json())? {
        return Err("Frozen instance does not match selected snapshot".into());
    }
    let recycle = match kind {
        Kind::RecycleBin => Some(
            library
                .entries()
                .iter()
                .filter(|record| record.entry_id != entry_id)
                .map(|record| record.original)
                .collect(),
        ),
        Kind::ReferenceLibrary => None,
    };
    append.finish(recycle, cancel)
}

// Permit only instantiation-owned identity/position/empty-parameter differences, preventing arbitrary windows from consuming another frozen record.
fn matches_snapshot(snapshot: &str, instance: &str) -> Result<bool, String> {
    let mut expected = candidate::fields(snapshot)?;
    let mut actual = candidate::fields(instance)?;
    for key in [
        "groupId",
        "artId",
        "inputs",
        "outputs",
        "executionConfig",
        "stickerEditPropagation",
    ] {
        expected.remove(key);
    }
    for key in ["id", "type", "x", "y", "params"] {
        expected.remove(key);
        actual.remove(key);
    }
    Ok(expected.len() == actual.len()
        && expected.iter().all(|(key, value)| {
            actual
                .get(key)
                .is_some_and(|actual| actual.get() == value.get())
        }))
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_library_retention_tests.rs"]
mod tests;
