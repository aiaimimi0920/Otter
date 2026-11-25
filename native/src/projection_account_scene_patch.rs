//! Original-text CAS for scene fields; borrow large PNG/unknown root fields without copying/deserializing the whole Base64 tree.
use super::*;
use serde::{ser::SerializeMap, Serializer};
use serde_json::value::RawValue;
use std::collections::BTreeMap;
struct Window<'a> {
    raw: &'a RawValue,
    replacement: Option<(BTreeMap<&'a str, &'a RawValue>, &'a Binding)>,
    png: Option<&'a str>,
}
impl Serialize for Window<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Some((fields, binding)) = &self.replacement else {
            return self.raw.serialize(serializer);
        };
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in fields {
            if *key != FIELD
                && !(self.png.is_some()
                    && matches!(
                        *key,
                        "src" | "filePath" | "previewSrc" | "outputs" | "restoredPreviewLocked"
                    ))
            {
                map.serialize_entry(key, value)?;
            }
        }
        map.serialize_entry(FIELD, binding)?;
        if let Some(png) = self.png {
            map.serialize_entry("src", png)?;
            map.serialize_entry("restoredPreviewLocked", &false)?;
        }
        map.end()
    }
}
struct Patch<'a> {
    root: BTreeMap<&'a str, &'a RawValue>,
    windows: Vec<Window<'a>>,
}
impl Serialize for Patch<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.root.len()))?;
        for (key, value) in &self.root {
            if *key == "stickers" {
                map.serialize_entry(key, &self.windows)?;
            } else {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}
pub(super) fn replace(
    source: &Source,
    next: &Binding,
    previous: Option<&Binding>,
    png: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Source, Error> {
    let parsed = source.parse().map_err(|_| Error::Response)?;
    if crate::session_projection_scope::Proof::capture_parsed(&parsed)
        .is_none_or(|proof| proof.scope() != next.scope)
    {
        return Err(Error::Response);
    }
    let root: BTreeMap<&str, &RawValue> =
        serde_json::from_slice(source).map_err(|_| Error::Response)?;
    let rows: Vec<&RawValue> =
        serde_json::from_str(root.get("stickers").ok_or(Error::Response)?.get())
            .map_err(|_| Error::Response)?;
    let mut windows = Vec::with_capacity(rows.len());
    let mut found = false;
    for raw in rows {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Stopped);
        }
        let fields: BTreeMap<&str, &RawValue> =
            serde_json::from_str(raw.get()).map_err(|_| Error::Response)?;
        let unit: &str = serde_json::from_str(fields.get("id").ok_or(Error::Response)?.get())
            .map_err(|_| Error::Response)?;
        let replacement = if unit == next.unit() {
            if found {
                return Err(Error::Response);
            }
            found = true;
            let old = fields
                .get(FIELD)
                .filter(|v| v.get() != "null")
                .map(|raw| Binding::decode(raw.get(), unit, next.scope))
                .transpose()?;
            if old.as_ref() != previous {
                return Err(Error::Response);
            }
            Some((fields, next))
        } else {
            None
        };
        let png = replacement.as_ref().and(png);
        windows.push(Window {
            raw,
            replacement,
            png,
        });
    }
    if !found || parsed.document().workspace.units().len() != windows.len() {
        return Err(Error::Response);
    }
    let bytes =
        crate::session_write::encode_bounded(&Patch { root, windows }, cancel).map_err(|_| {
            if cancel.load(std::sync::atomic::Ordering::Acquire) {
                Error::Stopped
            } else {
                Error::Budget
            }
        })?;
    crate::session_document::decode(&bytes).map_err(|_| Error::Response)?;
    Ok(bytes.into())
}
