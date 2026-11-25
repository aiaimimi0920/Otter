//! Read-only admission for session frozen libraries; preserve complete original entries rather than drop unknown fields and overwrite old sessions.
use crate::{session_window::Window, workspace_id::UnitId};
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::HashSet};
#[path = "session_library_candidate.rs"]
pub mod candidate;
#[path = "session_library_freeze.rs"]
mod freeze;
#[path = "session_library_prune.rs"]
pub mod prune;
#[path = "session_library_recycle.rs"]
pub mod recycle;
#[path = "session_library_reference.rs"]
pub mod reference;
#[path = "session_library_remove.rs"]
pub mod remove;
#[path = "session_library_retention.rs"]
pub mod retention;

// This is a single-import safety limit, not recycle-bin retention policy; reject the entire library on overflow, never silently trim entries.
pub const MAX_ENTRIES: usize = 4096;
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    RecycleBin,
    ReferenceLibrary,
}
pub struct Library<'a> {
    entries: Vec<Entry<'a>>,
}
pub struct Entry<'a> {
    pub entry_id: Cow<'a, str>,
    pub source_sticker_id: Cow<'a, str>,
    pub created_at: Cow<'a, str>,
    pub window: Window<'a>,
    original: &'a RawValue,
    snapshot: &'a RawValue,
}
impl Library<'_> {
    pub fn entries(&self) -> &[Entry<'_>] {
        &self.entries
    }
    pub fn find(&self, entry_id: &str) -> Option<&Entry<'_>> {
        self.entries.iter().find(|entry| entry.entry_id == entry_id)
    }
}
impl<'a> Entry<'a> {
    pub fn original_json(&self) -> &'a str {
        self.original.get()
    }
    pub fn snapshot_json(&self) -> &'a str {
        self.snapshot.get()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<'a> {
    #[serde(default)]
    document_schema_version: u32,
    #[serde(default, borrow)]
    recycle_bin: Option<&'a RawValue>,
    #[serde(default, borrow)]
    reference_library: Option<&'a RawValue>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record<'a> {
    #[serde(borrow)]
    entry_id: Cow<'a, str>,
    #[serde(borrow)]
    source_sticker_id: Cow<'a, str>,
    #[serde(borrow)]
    created_at: Cow<'a, str>,
    #[serde(borrow)]
    snapshot: &'a RawValue,
}
#[derive(Deserialize)]
struct Records<'a>(#[serde(borrow, deserialize_with = "records")] Vec<&'a RawValue>);
fn records<'de: 'a, 'a, D: Deserializer<'de>>(decoder: D) -> Result<Vec<&'a RawValue>, D::Error> {
    crate::session_document::bounded::<D, &'a RawValue, MAX_ENTRIES>(decoder)
}

pub fn decode(bytes: &[u8], kind: Kind) -> Result<Library<'_>, String> {
    if bytes.len() > crate::session_document::MAX_SESSION_BYTES {
        return Err("Frozen library session exceeds byte budget".into());
    }
    let envelope: Envelope = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if envelope.document_schema_version > 1 {
        return Err("Unsupported frozen library session schema".into());
    }
    let selected = match kind {
        Kind::RecycleBin => envelope.recycle_bin,
        Kind::ReferenceLibrary => envelope.reference_library,
    };
    let Some(selected) = selected else {
        return Ok(Library {
            entries: Vec::new(),
        });
    };
    let records: Records =
        serde_json::from_str(selected.get()).map_err(|error| error.to_string())?;
    let mut entries = Vec::with_capacity(records.0.len());
    let mut identities = HashSet::new();
    for original in records.0 {
        let record: Record =
            serde_json::from_str(original.get()).map_err(|error| error.to_string())?;
        if UnitId::parse(&record.entry_id).is_none()
            || UnitId::parse(&record.source_sticker_id).is_none()
            || !identities.insert(record.entry_id.to_string())
        {
            return Err("Invalid or duplicate frozen entry identity".into());
        }
        // Old frontends retain unparseable timestamps; limit length only, without timezone guessing/record deletion while reading.
        if record.created_at.len() > 128 || record.created_at.chars().any(char::is_control) {
            return Err("Invalid frozen entry timestamp".into());
        }
        let window: Window =
            serde_json::from_str(record.snapshot.get()).map_err(|error| error.to_string())?;
        if !window.valid()
            || window.id != record.source_sticker_id
            || window
                .node_type
                .as_deref()
                .is_some_and(|kind| kind != "sticker")
        {
            return Err("Invalid frozen sticker snapshot".into());
        }
        entries.push(Entry {
            entry_id: record.entry_id,
            source_sticker_id: record.source_sticker_id,
            created_at: record.created_at,
            window,
            original,
            snapshot: record.snapshot,
        });
    }
    Ok(Library { entries })
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_library_tests.rs"]
mod tests;
