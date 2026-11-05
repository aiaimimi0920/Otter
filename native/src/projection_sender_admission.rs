//! 首发副作用之前检查旧会话的来源/接收角色/完整容量；不能先 Create 再发现无处保存。
use super::*;
use std::borrow::Cow;
impl Binding {
    pub(crate) fn admit_update(source: &Source, expected: &Self) -> Result<(), Error> {
        let index = checkpoint::Index::capture(source).ok_or(Error::Format)?;
        if expected.stopped() || !index.includes(expected) {
            return Err(Error::Changed);
        }
        let root = fields(std::str::from_utf8(source).map_err(|_| Error::Format)?)
            .map_err(|_| Error::Format)?;
        let raw: Vec<&serde_json::value::RawValue> =
            serde_json::from_str(root.get("stickers").ok_or(Error::Format)?.get())
                .map_err(|_| Error::Format)?;
        for window in raw {
            let object = fields(window.get()).map_err(|_| Error::Format)?;
            let unit: Cow<'_, str> =
                serde_json::from_str(object.get("id").ok_or(Error::Format)?.get())
                    .map_err(|_| Error::Format)?;
            if unit == expected.unit() {
                return source_role(&object);
            }
        }
        Err(Error::Changed)
    }
    pub(crate) fn admit_send(source: &Source, selection: &Selection) -> Result<(), Error> {
        Self::admit_batch(source, std::slice::from_ref(selection))
    }
    /// 同一来源批次在第一份签名/网络副作用前核完整容量；每个成员仍需原单目标准入。
    pub(crate) fn admit_batch(source: &Source, selections: &[Selection]) -> Result<(), Error> {
        let selection = selections.first().ok_or(Error::Format)?;
        if selections.len() > MAX_PER_UNIT
            || selections.iter().any(|next| {
                next.scope != selection.scope
                    || next.unit != selection.unit
                    || next.origin != selection.origin
            })
        {
            return Err(Error::Limit);
        }
        let destinations: std::collections::BTreeSet<_> =
            selections.iter().map(Selection::destination).collect();
        if destinations.len() != selections.len() {
            return Err(Error::Changed);
        }
        let document = crate::session_document::decode(source).map_err(|_| Error::Format)?;
        if document
            .projection_scope
            .map(|scope| scope.to_string())
            .as_deref()
            != Some(&selection.scope)
        {
            return Err(Error::Changed);
        }
        let root = fields(std::str::from_utf8(source).map_err(|_| Error::Format)?)
            .map_err(|_| Error::Format)?;
        let raw: Vec<&serde_json::value::RawValue> =
            serde_json::from_str(root.get("stickers").ok_or(Error::Format)?.get())
                .map_err(|_| Error::Format)?;
        for window in raw {
            let object = fields(window.get()).map_err(|_| Error::Format)?;
            let unit: Cow<'_, str> =
                serde_json::from_str(object.get("id").ok_or(Error::Format)?.get())
                    .map_err(|_| Error::Format)?;
            if unit != selection.unit {
                continue;
            }
            source_role(&object)?;
            let entries = object
                .get("projectionSenders")
                .map(|raw| {
                    serde_json::from_str::<inventory::Entries<'_>>(raw.get())
                        .map_err(|_| Error::Format)
                })
                .transpose()?;
            let mut keys: Vec<inventory::Key> = Vec::new();
            for raw in entries.into_iter().flat_map(|entries| entries.0) {
                let key = inventory::key(raw.get(), &unit)?;
                if keys.iter().any(|old| {
                    (old.origin == key.origin && old.projection == key.projection)
                        || old.destination == key.destination
                }) || destinations.contains(&key.destination)
                {
                    return Err(Error::Changed);
                }
                keys.push(key);
            }
            let (primary, _) = inventory::primary(&object, &unit, &keys)?;
            return if keys.len() + primary + selections.len() <= MAX_PER_UNIT {
                Ok(())
            } else {
                Err(Error::Limit)
            };
        }
        Err(Error::Changed)
    }
}
fn source_role(
    object: &std::collections::BTreeMap<String, Cow<'_, serde_json::value::RawValue>>,
) -> Result<(), Error> {
    if let Some(primary) = object.get("qrProjection").filter(|raw| raw.get() != "null") {
        if primary.get().len() > MAX_BINDING_BYTES {
            return Err(Error::Limit);
        }
        let primary = fields(primary.get()).map_err(|_| Error::Format)?;
        let role: Cow<'_, str> =
            serde_json::from_str(primary.get("role").ok_or(Error::Format)?.get())
                .map_err(|_| Error::Format)?;
        if role == "receiver" {
            return Err(Error::Changed);
        }
    }
    Ok(())
}
#[cfg(test)]
#[path = "projection_sender_batch_admission_tests.rs"]
mod tests;
