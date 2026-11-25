//! Favorites/recycling share the current encoded-window freezing boundary; borrow original text without copying images/runtime ownership.
use super::candidate;
use crate::{session_window::Window, workspace_id::UnitId};
use serde::{ser::SerializeSeq, Serialize, Serializer};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap};
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Entry<'a> {
    entry_id: &'a str,
    source_sticker_id: &'a str,
    created_at: &'a str,
    snapshot: BTreeMap<String, Cow<'a, RawValue>>,
}
pub(super) fn capture<'a>(
    snapshot: &'a [u8],
    unit: &'a UnitId,
    entry_id: &'a str,
    created_at: &'a str,
) -> Result<Entry<'a>, String> {
    if UnitId::parse(entry_id).is_none()
        || created_at.is_empty()
        || created_at.len() > 128
        || created_at.chars().any(char::is_control)
    {
        return Err("Invalid frozen identity or timestamp".into());
    }
    if snapshot.len() > crate::session_document::MAX_SESSION_BYTES {
        return Err("Frozen sticker exceeds byte budget".into());
    }
    let window: Window<'_> = serde_json::from_slice(snapshot).map_err(|error| error.to_string())?;
    if window.id != unit.as_str() || !window.valid() {
        return Err("Invalid frozen source identity or geometry".into());
    }
    if window
        .node_type
        .as_deref()
        .is_some_and(|kind| kind != "sticker")
    {
        return Err("Only stickers can be frozen".into());
    }
    let mut snapshot =
        candidate::fields(std::str::from_utf8(snapshot).map_err(|error| error.to_string())?)?;
    for key in [
        "groupId",
        "artId",
        "params",
        "inputs",
        "outputs",
        "executionConfig",
        "stickerEditPropagation",
        "portVisibility",
        "otterNativeProjectionEdit",
    ] {
        snapshot.remove(key);
    }
    Ok(Entry {
        entry_id,
        source_sticker_id: unit.as_str(),
        created_at,
        snapshot,
    })
}

/// Both libraries retain existing entries as raw JSON; only a new entry is serialized.
pub(super) struct Entries<'a> {
    pub(super) retained: Vec<&'a RawValue>,
    pub(super) next: Option<Entry<'a>>,
}
impl Serialize for Entries<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer
            .serialize_seq(Some(self.retained.len() + usize::from(self.next.is_some())))?;
        for entry in &self.retained {
            sequence.serialize_element(entry)?;
        }
        if let Some(next) = &self.next {
            sequence.serialize_element(next)?;
        }
        sequence.end()
    }
}
