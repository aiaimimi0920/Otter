//! 关闭前的回收候选：当前快照入库，裁掉旧活动窗口及关联链接；发布权仍属于窗口协调器。
use super::{candidate, freeze, Kind, MAX_ENTRIES};
use crate::{pixels::Pixels, session_document, workspace_id::UnitId};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};
#[path = "session_library_recycle_policy.rs"]
pub mod policy;
pub struct Request<'a> {
    pub snapshot: &'a [u8],
    pub unit: &'a UnitId,
    pub entry_id: &'a str,
    pub created_at: &'a str,
}
struct Entries<'a> {
    retained: Vec<&'a RawValue>,
    next: Option<freeze::Entry<'a>>,
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
    root: &'a BTreeMap<String, Cow<'a, RawValue>>,
    stickers: Vec<&'a RawValue>,
    links: Vec<&'a RawValue>,
    entries: Entries<'a>,
}
impl Serialize for Updated<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in self.root {
            match key.as_str() {
                "stickers" => map.serialize_entry(key, &self.stickers)?,
                "links" => map.serialize_entry(key, &self.links)?,
                "recycleBin" => {}
                _ => map.serialize_entry(key, value)?,
            }
        }
        map.serialize_entry("recycleBin", &self.entries)?;
        map.end()
    }
}
pub fn prepare(
    original: &[u8],
    request: Request<'_>,
    policy: policy::Policy,
    now_ms: i64,
    cancel: &AtomicBool,
) -> Result<Pixels, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Recycle preparation cancelled".into());
    }
    let policy = policy.validate()?;
    let source = session_document::decode(original)
        .map_err(|error| format!("Invalid recycle source: {error:?}"))?;
    let library = super::decode(original, Kind::RecycleBin)?;
    if library.find(request.entry_id).is_some() {
        return Err("Recycle entry identity conflict".into());
    }
    let next = freeze::capture(
        request.snapshot,
        request.unit,
        request.entry_id,
        request.created_at,
    )?;
    let next = policy.retains(request.created_at, now_ms).then_some(next);
    let mut retained: Vec<_> = library
        .entries()
        .iter()
        .filter(|entry| policy.retains(&entry.created_at, now_ms))
        .map(|entry| entry.original)
        .collect();
    let count = retained.len() + usize::from(next.is_some());
    if policy.recycle_bin_max_entries != 0 {
        retained.drain(..count.saturating_sub(policy.recycle_bin_max_entries));
    }
    if retained.len() + usize::from(next.is_some()) > MAX_ENTRIES {
        return Err("Recycle library exceeds hard entry budget".into());
    }
    let root =
        candidate::fields(std::str::from_utf8(original).map_err(|error| error.to_string())?)?;
    if root.len() == 256 && !root.contains_key("recycleBin") {
        return Err("Recycle root field budget exceeded".into());
    }
    let raw_windows: Vec<&RawValue> =
        serde_json::from_str(root["stickers"].get()).map_err(|error| error.to_string())?;
    let stickers = raw_windows
        .into_iter()
        .zip(&source.windows)
        .filter(|(_, window)| window.id != request.unit.as_str())
        .map(|(raw, _)| raw)
        .collect();
    let raw_links: Vec<&RawValue> =
        serde_json::from_str(root["links"].get()).map_err(|error| error.to_string())?;
    let links = raw_links
        .into_iter()
        .zip(source.workspace.links())
        .filter(|(_, link)| &link.from_unit != request.unit && &link.to_unit != request.unit)
        .map(|(raw, _)| raw)
        .collect();
    let bytes = crate::session_write::encode_bounded(
        &Updated {
            root: &root,
            stickers,
            links,
            entries: Entries { retained, next },
        },
        cancel,
    )
    .map_err(|error| format!("Recycle encoding: {error:?}"))?;
    session_document::decode(&bytes)
        .map_err(|error| format!("Invalid recycle result: {error:?}"))?;
    super::decode(&bytes, Kind::RecycleBin)?;
    Ok(bytes)
}

#[cfg(test)]
#[path = "session_library_recycle_tests.rs"]
mod tests;
