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
#[cfg(windows)]
pub(crate) struct Target<'a> {
    pub original: &'a [u8],
    pub active: &'a [UnitId],
}
#[cfg(windows)]
impl Target<'_> {
    pub fn append(
        self,
        copy: &session_document::Document<'_>,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels, String> {
        let source = decode_source(self.original)?;
        Prepared::new(&source, copy, self.active)?.finish(None, cancel)
    }
}
#[cfg(any(windows, test))]
fn decode_source(original: &[u8]) -> Result<session_document::Document<'_>, String> {
    session_document::decode(original).map_err(|error| format!("Invalid source: {error:?}"))
}

/// The exact sticker used for snapshot comparison is also the one serialized by `finish`.
pub(crate) struct Prepared<'a> {
    source: &'a session_document::Document<'a>,
    active: &'a [UnitId],
    unit_id: &'a str,
    sticker: &'a RawValue,
}
impl<'a> Prepared<'a> {
    pub fn new(
        source: &'a session_document::Document<'a>,
        copy: &'a session_document::Document<'a>,
        active: &'a [UnitId],
    ) -> Result<Self, String> {
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
        let Some(Cow::Borrowed(stickers)) =
            fields(std::str::from_utf8(copy.original_json()).map_err(|error| error.to_string())?)?
                .remove("stickers")
        else {
            return Err("Missing instance stickers".into());
        };
        let [sticker]: [&RawValue; 1] =
            serde_json::from_str(stickers.get()).map_err(|error| error.to_string())?;
        Ok(Self {
            source,
            active,
            unit_id: &window.id,
            sticker,
        })
    }

    pub fn unit_id(&self) -> &str {
        self.unit_id
    }

    pub fn sticker_json(&self) -> &str {
        self.sticker.get()
    }

    pub fn finish(
        self,
        recycle: Option<Vec<&RawValue>>,
        cancel: &AtomicBool,
    ) -> Result<crate::pixels::Pixels, String> {
        let Self {
            source,
            active,
            sticker,
            ..
        } = self;
        let root = fields(
            std::str::from_utf8(source.original_json()).map_err(|error| error.to_string())?,
        )?;
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
        stickers.push(sticker);
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
}
#[cfg(test)]
#[path = "session_append_tests.rs"]
mod tests;
