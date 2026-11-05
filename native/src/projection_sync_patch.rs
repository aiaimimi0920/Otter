//! 后台原文 source/binding 事务；只借用大型未知字段，不重建本地编辑与几何数据。
use crate::{
    projection_delivery::binding::Binding, session_json_fields::fields, session_source::Source,
};
use serde::{ser::SerializeMap, Serialize, Serializer};
use serde_json::value::RawValue;
use std::{borrow::Cow, collections::BTreeMap, sync::atomic::AtomicBool};

struct Sticker<'a> {
    raw: &'a RawValue,
    patch: Option<Replacement<'a>>,
}
struct Replacement<'a> {
    fields: BTreeMap<String, Cow<'a, RawValue>>,
    binding: &'a Binding,
    png: Option<&'a str>,
}
impl Serialize for Sticker<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Some(Replacement {
            fields,
            binding,
            png,
        }) = &self.patch
        else {
            return self.raw.serialize(serializer);
        };
        let mut map = serializer.serialize_map(None)?;
        for (key, raw) in fields {
            if key == "qrProjection"
                || png.is_some()
                    && matches!(
                        key.as_str(),
                        "src" | "filePath" | "previewSrc" | "outputs" | "restoredPreviewLocked"
                    )
            {
                continue;
            }
            map.serialize_entry(key, raw)?;
        }
        map.serialize_entry("qrProjection", binding)?;
        if let Some(png) = png {
            map.serialize_entry("src", png)?;
            map.serialize_entry("restoredPreviewLocked", &false)?;
        }
        map.end()
    }
}
struct Patch<'a> {
    root: &'a BTreeMap<String, Cow<'a, RawValue>>,
    stickers: Vec<Sticker<'a>>,
}
impl Serialize for Patch<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.root.len()))?;
        for (key, raw) in self.root {
            if key == "stickers" {
                map.serialize_entry(key, &self.stickers)?;
            } else {
                map.serialize_entry(key, raw)?;
            }
        }
        map.end()
    }
}
pub(crate) fn prepare(
    source: &Source,
    expected: &Binding,
    next: &Binding,
    png: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Source, String> {
    let check = || {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            Err("Projection patch cancelled".to_owned())
        } else {
            Ok(())
        }
    };
    check()?;
    let index = crate::projection_delivery::binding::checkpoint::Index::capture(source)
        .ok_or("Invalid binding index")?;
    if !index.includes(expected) || next.unit() != expected.unit() {
        return Err("Changed projection binding".into());
    }
    let root = fields(std::str::from_utf8(source).map_err(|e| e.to_string())?)?;
    let raw: Vec<&RawValue> =
        serde_json::from_str(root.get("stickers").ok_or("Missing stickers")?.get())
            .map_err(|e| e.to_string())?;
    let mut stickers = Vec::with_capacity(raw.len());
    for raw in raw {
        check()?;
        let object = fields(raw.get())?;
        let unit: &str = serde_json::from_str(object.get("id").ok_or("Missing unit")?.get())
            .map_err(|e| e.to_string())?;
        let patch = (unit == expected.unit()).then_some(Replacement {
            fields: object,
            binding: next,
            png,
        });
        stickers.push(Sticker { raw, patch });
    }
    let encoded = crate::session_write::encode_bounded(
        &Patch {
            root: &root,
            stickers,
        },
        cancel,
    )
    .map_err(|e| format!("Projection patch: {e:?}"))?;
    crate::session_document::decode(&encoded).map_err(|e| format!("Projection patch: {e:?}"))?;
    check()?;
    Ok(encoded.into())
}
