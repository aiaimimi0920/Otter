//! 按条目身份删除一个冻结记录；不解析图片、不更改活动窗口或其它库，也不推进 revision。
use super::{candidate, Kind};
use crate::session_write::Candidate;
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
) -> Result<Candidate, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Frozen removal cancelled".into());
    }
    if crate::workspace_id::UnitId::parse(entry_id).is_none() {
        return Err("Invalid frozen entry identity".into());
    }
    let revision = source_revision(original)?;
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
    Ok(Candidate {
        bytes,
        expected_revision: revision,
        revision,
    })
}

/// 显式清空只修改选定库的原文；不删除图片文件、不改变另一库或活动窗口。
pub fn clear(
    original: &[u8],
    kind: Kind,
    cancel: &AtomicBool,
) -> Result<Option<Candidate>, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Frozen clear cancelled".into());
    }
    let revision = source_revision(original)?;
    let library = super::decode(original, kind)?;
    if library.entries().is_empty() {
        // 缺失或已空的库不发布任何字节，包括保留导入原文的首尾空白。
        return Ok(None);
    }
    let root =
        candidate::fields(std::str::from_utf8(original).map_err(|error| error.to_string())?)?;
    let key = match kind {
        Kind::RecycleBin => "recycleBin",
        Kind::ReferenceLibrary => "referenceLibrary",
    };
    let bytes = crate::session_write::encode_bounded(
        &Updated {
            root,
            key,
            entries: Vec::new(),
        },
        cancel,
    )
    .map_err(|error| format!("Frozen clear encoding: {error:?}"))?;
    crate::session_document::decode(&bytes)
        .map_err(|error| format!("Invalid frozen clear: {error:?}"))?;
    Ok(Some(Candidate {
        bytes,
        expected_revision: revision,
        revision,
    }))
}

fn source_revision(original: &[u8]) -> Result<u64, String> {
    crate::session_document::decode(original)
        .map(|document| document.revision)
        .map_err(|error| format!("Invalid frozen source: {error:?}"))
}

#[cfg(all(test, any(not(feature = "scoped-tests"), feature = "test-save")))]
#[path = "session_library_remove_tests.rs"]
mod tests;
