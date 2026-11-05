//! 后台原文 patch：大型图片/未知根字段均借用，更新关联时保留小对象的未知扩展。
use super::*;
use serde::{ser::SerializeMap, Serializer};
use serde_json::{value::RawValue, Value};
use std::{borrow::Cow, collections::BTreeMap};
struct Window<'a> {
    raw: &'a RawValue,
    patch: Option<Replacement<'a>>,
}
struct Replacement<'a> {
    fields: BTreeMap<String, Cow<'a, RawValue>>,
    bindings: Vec<Value>,
}
impl Serialize for Window<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Some(Replacement { fields, bindings }) = &self.patch else {
            return self.raw.serialize(serializer);
        };
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in fields {
            if key != "projectionSenders" {
                map.serialize_entry(key, value)?;
            }
        }
        map.serialize_entry("projectionSenders", bindings)?;
        map.end()
    }
}
struct Patch<'a> {
    root: &'a BTreeMap<String, Cow<'a, RawValue>>,
    windows: Vec<Window<'a>>,
}
impl Serialize for Patch<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.root.len()))?;
        for (key, value) in self.root {
            if key == "stickers" {
                map.serialize_entry(key, &self.windows)?;
            } else {
                map.serialize_entry(key, value)?;
            }
        }
        map.end()
    }
}
fn merge(
    object: &BTreeMap<String, Cow<'_, RawValue>>,
    next: &Binding,
) -> Result<Vec<Value>, Error> {
    let entries = match object.get("projectionSenders") {
        None => Vec::new(),
        Some(raw) => {
            serde_json::from_str::<inventory::Entries<'_>>(raw.get())
                .map_err(|_| Error::Format)?
                .0
        }
    };
    let mut keys: Vec<inventory::Key> = Vec::new();
    let mut result = Vec::with_capacity(entries.len().saturating_add(1));
    let destination = next.destination();
    let mut updated = false;
    for raw in entries {
        let key = inventory::key(raw.get(), next.unit())?;
        if keys.iter().any(|old| {
            (old.origin == key.origin && old.projection == key.projection)
                || old.destination == key.destination
        }) {
            return Err(Error::Changed);
        }
        let mut value: Value = serde_json::from_str(raw.get()).map_err(|_| Error::Format)?;
        if key.destination == destination {
            let old = Binding::decode(raw.get(), next.unit(), &next.link.otter_native_send.scope)?;
            if old.envelope() != next.envelope()
                || old.stopped()
                || next.revision() < old.revision()
                || (next.revision() == old.revision() && next.digest() != old.digest())
            {
                return Err(Error::Changed);
            }
            value["link"]["revision"] = next.revision().into();
            value["link"]["digest"] = next.digest().into();
            value["link"]["linked"] = next.linked().into();
            value["target"]["name"] = next.target.name.clone().into();
            updated = true;
        } else if key.origin == next.envelope().origin()
            && key.projection == next.envelope().projection_id()
        {
            return Err(Error::Changed);
        }
        keys.push(key);
        result.push(value);
    }
    let (primary, envelope) = inventory::primary(object, next.unit(), &keys)?;
    if envelope.as_ref().is_some_and(|envelope| {
        envelope.origin() == next.envelope().origin()
            && envelope.projection_id() == next.envelope().projection_id()
    }) {
        return Err(Error::Changed);
    }
    if !updated {
        if result.len() + primary >= MAX_PER_UNIT {
            return Err(Error::Limit);
        }
        result.push(serde_json::to_value(next).map_err(|_| Error::Format)?);
    }
    Ok(result)
}
pub(super) fn prepare(
    source: &Source,
    next: &Binding,
    cancel: &AtomicBool,
) -> Result<Source, Error> {
    transform(source, next.unit(), cancel, |object| merge(object, next))
}
pub(super) fn transform(
    source: &Source,
    target: &str,
    cancel: &AtomicBool,
    mut change: impl FnMut(&BTreeMap<String, Cow<'_, RawValue>>) -> Result<Vec<Value>, Error>,
) -> Result<Source, Error> {
    let root = fields(std::str::from_utf8(source).map_err(|_| Error::Format)?)
        .map_err(|_| Error::Format)?;
    let raw: Vec<&RawValue> =
        serde_json::from_str(root.get("stickers").ok_or(Error::Format)?.get())
            .map_err(|_| Error::Format)?;
    let mut windows = Vec::with_capacity(raw.len());
    let mut found = false;
    for raw in raw {
        check(cancel)?;
        let object = fields(raw.get()).map_err(|_| Error::Format)?;
        let unit: Cow<'_, str> = serde_json::from_str(object.get("id").ok_or(Error::Format)?.get())
            .map_err(|_| Error::Format)?;
        let patch = if unit == target {
            if found {
                return Err(Error::Changed);
            }
            found = true;
            let bindings = change(&object)?;
            Some(Replacement {
                fields: object,
                bindings,
            })
        } else {
            None
        };
        windows.push(Window { raw, patch });
    }
    if !found {
        return Err(Error::Changed);
    }
    let bytes = crate::session_write::encode_bounded(
        &Patch {
            root: &root,
            windows,
        },
        cancel,
    )
    .map_err(|_| Error::Limit)?;
    crate::session_document::decode(&bytes).map_err(|_| Error::Format)?;
    check(cancel)?;
    Ok(bytes.into())
}
