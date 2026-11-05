//! 已有回收站的后台裁剪；只改回收数组，未知条目值/其他库/活动窗口与 revision 保留。
use super::{candidate, recycle::policy::Policy, Kind};
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

struct Updated<'a> {
    root: BTreeMap<String, Cow<'a, RawValue>>,
    retained: Vec<&'a RawValue>,
}
impl Serialize for Updated<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.root.len()))?;
        for (key, value) in &self.root {
            if key == "recycleBin" {
                map.serialize_entry(key, &self.retained)?;
            } else {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}
pub fn prepare(
    original: &[u8],
    policy: Policy,
    now_ms: i64,
    cancel: &AtomicBool,
) -> Result<Option<crate::pixels::Pixels>, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Recycle pruning cancelled".into());
    }
    let policy = policy.validate()?;
    crate::session_document::decode(original)
        .map_err(|error| format!("Invalid prune source: {error:?}"))?;
    let library = super::decode(original, Kind::RecycleBin)?;
    let mut retained: Vec<_> = library
        .entries()
        .iter()
        .filter(|entry| policy.retains(&entry.created_at, now_ms))
        .map(|entry| entry.original)
        .collect();
    if policy.recycle_bin_max_entries != 0 {
        retained.drain(
            ..retained
                .len()
                .saturating_sub(policy.recycle_bin_max_entries),
        );
    }
    if retained.len() == library.entries().len() {
        return Ok(None);
    }
    let root =
        candidate::fields(std::str::from_utf8(original).map_err(|error| error.to_string())?)?;
    let bytes = crate::session_write::encode_bounded(&Updated { root, retained }, cancel)
        .map_err(|error| format!("Recycle prune encoding: {error:?}"))?;
    super::decode(&bytes, Kind::RecycleBin)?;
    Ok(Some(bytes))
}

#[cfg(test)]
#[path = "session_library_prune_tests.rs"]
mod tests;
