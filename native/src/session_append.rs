//! 单贴纸追加的有界原文事务；保留未知字段，裁掉已关闭窗口，不推进磁盘修订。
use crate::{session_document, session_json_fields::fields, workspace_id::UnitId};
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap, sync::atomic::AtomicBool};
struct Retained<'a> {
    root: &'a BTreeMap<String, Cow<'a, RawValue>>,
    stickers: Vec<&'a RawValue>,
    links: Vec<&'a RawValue>,
    recycle: Option<Vec<&'a RawValue>>,
}
impl Serialize for Retained<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.root.len()))?;
        for (key, value) in self.root {
            if key == "stickers" {
                map.serialize_entry(key, &self.stickers)?;
            } else if key == "links" {
                map.serialize_entry(key, &self.links)?;
            } else if let ("recycleBin", Some(recycle)) = (key.as_str(), &self.recycle) {
                map.serialize_entry(key, recycle)?;
            } else {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}
pub fn prepare(
    original: &[u8],
    instance: &[u8],
    active: &[UnitId],
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    prepare_with_recycle(original, instance, active, None, cancel)
}
pub(crate) fn prepare_with_recycle<'a>(
    original: &'a [u8],
    instance: &'a [u8],
    active: &[UnitId],
    recycle: Option<Vec<&'a RawValue>>,
    cancel: &AtomicBool,
) -> Result<crate::pixels::Pixels, String> {
    let source =
        session_document::decode(original).map_err(|error| format!("Invalid source: {error:?}"))?;
    let copy = session_document::decode(instance)
        .map_err(|error| format!("Invalid instance: {error:?}"))?;
    if copy.windows.len() != 1
        || !copy.workspace.links().is_empty()
        || !copy.workspace.groups().is_empty()
    {
        return Err("Append requires one ungrouped sticker".into());
    }
    let window = &copy.windows[0];
    if window.node_type.as_deref() != Some("sticker")
        || window.group_id.is_some()
        || window.params.is_none_or(|params| params.get() != "{}")
        || active.iter().any(|unit| unit.as_str() == window.id)
        || source.windows.iter().any(|old| old.id == window.id)
        || active.len() >= crate::workspace::MAX_UNITS
    {
        return Err("Append identity or capacity conflict".into());
    }
    let root = fields(std::str::from_utf8(original).map_err(|error| error.to_string())?)?;
    let copy_root = fields(std::str::from_utf8(instance).map_err(|error| error.to_string())?)?;
    let mut stickers: Vec<&RawValue> =
        serde_json::from_str(root.get("stickers").ok_or("Missing source stickers")?.get())
            .map_err(|error| error.to_string())?;
    let mut index = 0;
    stickers.retain(|_| {
        let keep = active
            .iter()
            .any(|unit| unit.as_str() == source.windows[index].id);
        index += 1;
        keep
    });
    let new: Vec<&RawValue> = serde_json::from_str(
        copy_root
            .get("stickers")
            .ok_or("Missing instance stickers")?
            .get(),
    )
    .map_err(|error| error.to_string())?;
    stickers.push(new[0]);
    let mut links: Vec<&RawValue> =
        serde_json::from_str(root.get("links").ok_or("Missing source links")?.get())
            .map_err(|error| error.to_string())?;
    let mut index = 0;
    links.retain(|_| {
        let link = &source.workspace.links()[index];
        index += 1;
        active.contains(&link.from_unit) && active.contains(&link.to_unit)
    });
    let bytes = crate::session_write::encode_bounded(
        &Retained {
            root: &root,
            stickers,
            links,
            recycle,
        },
        cancel,
    )
    .map_err(|error| format!("Append retention encoding: {error:?}"))?;
    session_document::decode(&bytes)
        .map_err(|error| format!("Invalid retained session: {error:?}"))?;
    Ok(bytes)
}
#[cfg(test)]
#[path = "session_append_tests.rs"]
mod tests;
