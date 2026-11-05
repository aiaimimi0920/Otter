//! 按条目身份删除一个冻结记录；不解析图片、不更改活动窗口或其它库，也不推进 revision。
use super::{candidate, Kind};
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::value::RawValue;
use std::{
    borrow::Cow,
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};
struct Updated<'a> {
    root: BTreeMap<String, Cow<'a, RawValue>>,
    key: &'static str,
    entries: Vec<&'a RawValue>,
}
impl Serialize for Updated<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.root.len()))?;
        for (key, value) in &self.root {
            if key == self.key {
                map.serialize_entry(key, &self.entries)?;
            } else {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}
pub fn prepare(
    original: &[u8],
    kind: Kind,
    entry_id: &str,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Frozen removal cancelled".into());
    }
    if crate::workspace_id::UnitId::parse(entry_id).is_none() {
        return Err("Invalid frozen entry identity".into());
    }
    crate::session_document::decode(original)
        .map_err(|error| format!("Invalid frozen source: {error:?}"))?;
    let library = super::decode(original, kind)?;
    if library.find(entry_id).is_none() {
        return Err("Frozen entry no longer exists".into());
    }
    let root =
        candidate::fields(std::str::from_utf8(original).map_err(|error| error.to_string())?)?;
    let key = match kind {
        Kind::RecycleBin => "recycleBin",
        Kind::ReferenceLibrary => "referenceLibrary",
    };
    let entries = library
        .entries()
        .iter()
        .filter(|entry| entry.entry_id != entry_id)
        .map(|entry| entry.original)
        .collect();
    let bytes = crate::session_write::encode_bounded(&Updated { root, key, entries }, cancel)
        .map_err(|error| format!("Frozen removal encoding: {error:?}"))?;
    crate::session_document::decode(&bytes)
        .map_err(|error| format!("Invalid frozen removal: {error:?}"))?;
    super::decode(&bytes, kind)?;
    Ok(bytes)
}

#[cfg(test)]
#[path = "session_library_remove_tests.rs"]
mod tests;
