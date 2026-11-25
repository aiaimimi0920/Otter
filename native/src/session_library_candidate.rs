//! Side-effect-free frozen-snapshot instantiation; output is only single-window preparation, never a whole-workspace replacement command.
use super::{Entry, Kind};
use crate::{session_document::Document, workspace_id::UnitId};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Prepared {
    pub unit: UnitId,
    /// Bounded independent single-window session, not a savable replacement for the original session.
    pub bytes: crate::pixels::Pixels,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a> {
    document_schema_version: u32,
    document_revision: u64,
    stickers: [BTreeMap<String, Cow<'a, RawValue>>; 1],
    links: [(); 0],
    groups: [(); 0],
    #[serde(skip_serializing_if = "Option::is_none")]
    otter_native_coordinate_space: Option<&'a RawValue>,
}

#[derive(Deserialize)]
struct Coordinates<'a> {
    #[serde(default, borrow, rename = "otterNativeCoordinateSpace")]
    value: Option<&'a RawValue>,
}

pub(super) use crate::session_json_fields::fields;

/// Callers supply fresh identity; publication still rechecks current workspace, since preparation-time checks are not locks.
/// Both libraries are read-only; later successful publication transactions consume recycle-bin entries, not deletion here.
pub fn prepare(
    source: &Document<'_>,
    kind: Kind,
    entry_id: &str,
    unit: UnitId,
    mouse: (f64, f64),
    cancel: &AtomicBool,
) -> Result<Prepared, String> {
    check(cancel)?;
    let library = source.library(kind)?;
    let entry = library.find(entry_id).ok_or("Frozen entry not found")?;
    let prepared = encode_selected(source, entry, unit, mouse, cancel)?;
    crate::session_document::decode(prepared.bytes.as_ref())
        .map_err(|error| format!("Invalid frozen candidate: {error:?}"))?;
    check(cancel)?;
    Ok(prepared)
}

/// Append workers already selected entries from the same source; pass encoded results directly to decoding that borrows original text.
pub(crate) fn encode_selected(
    source: &Document<'_>,
    entry: &Entry<'_>,
    unit: UnitId,
    mouse: (f64, f64),
    cancel: &AtomicBool,
) -> Result<Prepared, String> {
    check(cancel)?;
    if unit.as_str() == entry.source_sticker_id
        || source
            .windows
            .iter()
            .any(|window| window.id == unit.as_str())
    {
        return Err("Frozen copy requires a fresh unit identity".into());
    }
    let (x, y) = (mouse.0 + 50.0, mouse.1 + 50.0);
    let mut fields = fields(entry.snapshot_json())?;
    // Never inherit original-window groups, node ports, execution state, or edit-propagation ownership.
    for key in [
        "groupId",
        "artId",
        "inputs",
        "outputs",
        "executionConfig",
        "stickerEditPropagation",
    ] {
        fields.remove(key);
    }
    for (key, value) in [
        ("id", serde_json::json!(unit.as_str())),
        ("type", serde_json::json!("sticker")),
        ("x", serde_json::json!(x)),
        ("y", serde_json::json!(y)),
        ("params", serde_json::json!({})),
    ] {
        fields.insert(
            key.into(),
            Cow::Owned(serde_json::value::to_raw_value(&value).map_err(|error| error.to_string())?),
        );
    }
    if !x.is_finite() || !y.is_finite() {
        return Err("Invalid frozen copy position".into());
    }
    let root: Coordinates =
        serde_json::from_slice(source.original_json()).map_err(|error| error.to_string())?;
    let envelope = Envelope {
        document_schema_version: 1,
        document_revision: 0,
        stickers: [fields],
        links: [],
        groups: [],
        otter_native_coordinate_space: root.value,
    };
    let bytes = crate::session_write::encode_bounded(&envelope, cancel)
        .map_err(|error| format!("Frozen candidate encoding: {error:?}"))?;
    Ok(Prepared { unit, bytes })
}

fn check(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err("Frozen candidate cancelled".into())
    } else {
        Ok(())
    }
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_library_candidate_tests.rs"]
mod tests;
