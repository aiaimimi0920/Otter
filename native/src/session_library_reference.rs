//! 收藏写入只替换 referenceLibrary；快照来自已编码的当前模型，不推进磁盘 revision。
use super::{candidate, freeze::Entry, Kind, MAX_ENTRIES};
use crate::{pixels::Pixels, session_document, workspace_id::UnitId};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap, sync::atomic::AtomicBool};

struct Entries<'a> {
    retained: Vec<&'a RawValue>,
    next: Option<Entry<'a>>,
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
struct Updated<'a> {
    root: BTreeMap<String, Cow<'a, RawValue>>,
    entries: Entries<'a>,
}
impl Serialize for Updated<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in &self.root {
            if key != "referenceLibrary" {
                map.serialize_entry(key, value)?;
            }
        }
        map.serialize_entry("referenceLibrary", &self.entries)?;
        map.end()
    }
}

/// snapshot=None 是按源身份取消收藏，不要求重新编码图片。调用者在发布前验证来源和编辑凭证。
pub fn prepare(
    original: &[u8],
    snapshot: Option<&[u8]>,
    unit: &UnitId,
    entry_id: &str,
    created_at: &str,
    cancel: &AtomicBool,
) -> Result<Pixels, String> {
    if cancel.load(std::sync::atomic::Ordering::Acquire) {
        return Err("Reference update cancelled".into());
    }
    session_document::decode(original)
        .map_err(|error| format!("Invalid reference source: {error:?}"))?;
    let library = super::decode(original, Kind::ReferenceLibrary)?;
    let root =
        candidate::fields(std::str::from_utf8(original).map_err(|error| error.to_string())?)?;
    if root.len() == 256 && !root.contains_key("referenceLibrary") {
        return Err("Reference field would exceed the retained root field budget".into());
    }
    let retained: Vec<_> = library
        .entries()
        .iter()
        .filter(|entry| entry.source_sticker_id != unit.as_str())
        .map(|entry| entry.original)
        .collect();
    let next = if let Some(snapshot) = snapshot {
        if UnitId::parse(entry_id).is_none()
            || created_at.is_empty()
            || created_at.len() > 128
            || created_at.chars().any(char::is_control)
            || library
                .entries()
                .iter()
                .any(|entry| entry.entry_id == entry_id)
            || retained.len() >= MAX_ENTRIES
        {
            return Err("Invalid reference identity, timestamp or capacity".into());
        }
        Some(super::freeze::capture(
            snapshot, unit, entry_id, created_at,
        )?)
    } else {
        None
    };
    let bytes = crate::session_write::encode_bounded(
        &Updated {
            root,
            entries: Entries { retained, next },
        },
        cancel,
    )
    .map_err(|error| format!("Reference encoding: {error:?}"))?;
    session_document::decode(&bytes)
        .map_err(|error| format!("Invalid reference result: {error:?}"))?;
    super::decode(&bytes, Kind::ReferenceLibrary)?;
    Ok(bytes)
}

#[cfg(test)]
#[path = "session_library_reference_tests.rs"]
mod tests;
