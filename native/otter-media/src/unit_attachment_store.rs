//! 文档附件的不可变快照；显隐只改宿主标记，payload 和 CAS 版本保持不变。
use crate::{loom_extension_snapshot::Snapshot, unit_attachment::Attachment};
use serde::Serialize;
use serde_json::{value::RawValue, Map, Value};
use std::{collections::BTreeSet, sync::Arc};
const MAX: u64 = crate::loom_extension_snapshot::MAX_INTEGER;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Unavailable,
    Owner,
    Stale,
    Conflict,
    Limit,
}
#[derive(Clone, Default)]
pub struct Store {
    data: Data,
    identity: Arc<()>,
}
#[derive(Clone, Default)]
enum Data {
    #[default]
    Empty,
    Ready(Arc<Ready>),
    Unavailable(Arc<RawValue>),
}
#[derive(Clone)]
struct Ready {
    revision: u64,
    items: Vec<Stored>,
    extra: Arc<Map<String, Value>>,
}
#[derive(Clone)]
pub struct Stored {
    pub attachment: Attachment,
    pub digest: Option<Arc<str>>,
    pub visible: Option<bool>,
    extra: Arc<Map<String, Value>>,
}
#[derive(Clone)]
pub struct Stamp(Arc<()>);
impl Stamp {
    pub fn matches(&self, store: &Store) -> bool {
        Arc::ptr_eq(&self.0, &store.identity)
    }
}
impl Stored {
    fn parse(mut value: Value) -> Result<Self, Error> {
        let attachment = Attachment::parse(value.clone()).map_err(|_| Error::Invalid)?;
        let fields = value.as_object_mut().ok_or(Error::Invalid)?;
        let visible = fields
            .remove("overlayVisible")
            .map(|v| v.as_bool().ok_or(Error::Invalid))
            .transpose()?;
        let digest = fields
            .remove("payloadDigest")
            .map(|v| {
                v.as_str()
                    .filter(|s| crate::unit_attachment::valid_digest(s))
                    .map(Arc::<str>::from)
                    .ok_or(Error::Invalid)
            })
            .transpose()?;
        if let Some(expected) = &digest {
            if attachment.payload().is_none_or(|payload| {
                crate::unit_attachment_json::digest(payload) != expected.as_ref()
            }) {
                return Err(Error::Invalid);
            }
        }
        for key in [
            "attachmentId",
            "typeId",
            "schemaVersion",
            "revision",
            "pluginId",
            "pluginVersion",
            "rendererId",
            "payload",
            "resourceRefs",
            "priorRevision",
        ] {
            fields.remove(key);
        }
        Ok(Self {
            attachment,
            digest,
            visible,
            extra: Arc::new(std::mem::take(fields)),
        })
    }
    pub fn is_visible(&self) -> bool {
        self.visible.unwrap_or_else(|| {
            self.attachment
                .payload()
                .and_then(|p| p.get("visible"))
                .and_then(Value::as_bool)
                != Some(false)
        })
    }
    fn value(&self) -> Result<Value, Error> {
        let mut fields = (*self.extra).clone();
        let Value::Object(attachment) =
            serde_json::to_value(&self.attachment).map_err(|_| Error::Invalid)?
        else {
            unreachable!()
        };
        fields.extend(attachment);
        if let Some(digest) = &self.digest {
            fields.insert("payloadDigest".into(), Value::String(digest.to_string()));
        }
        if let Some(visible) = self.visible {
            fields.insert("overlayVisible".into(), visible.into());
        }
        Ok(Value::Object(fields))
    }
}
impl Store {
    /// 后台恢复边界；未知/损坏数据原样保留，但不成为活动扩展权限或可写附件。
    pub fn restore(raw: Option<&RawValue>) -> Self {
        let Some(raw) = raw else {
            return Self::default();
        };
        let data = Self::parse(raw).map_or_else(
            |_| Data::Unavailable(Arc::from(raw.to_owned())),
            |r| Data::Ready(Arc::new(r)),
        );
        Self {
            data,
            identity: Arc::new(()),
        }
    }
    fn parse(raw: &RawValue) -> Result<Ready, Error> {
        if raw.get().len() > 20 * 1024 * 1024 {
            return Err(Error::Limit);
        }
        let mut value: Value = serde_json::from_str(raw.get()).map_err(|_| Error::Invalid)?;
        let fields = value.as_object_mut().ok_or(Error::Invalid)?;
        if fields.remove("schemaVersion").and_then(|v| v.as_u64()) != Some(1) {
            return Err(Error::Invalid);
        }
        let revision = fields
            .remove("revision")
            .and_then(|v| v.as_u64())
            .filter(|v| *v <= MAX)
            .ok_or(Error::Invalid)?;
        let Value::Array(items) = fields.remove("attachments").ok_or(Error::Invalid)? else {
            return Err(Error::Invalid);
        };
        if items.len() > 64 {
            return Err(Error::Limit);
        }
        let items = items
            .into_iter()
            .map(Stored::parse)
            .collect::<Result<Vec<_>, _>>()?;
        let mut ids = BTreeSet::new();
        if items.iter().any(|item| !ids.insert(item.attachment.id())) {
            return Err(Error::Invalid);
        }
        Ok(Ready {
            revision,
            items,
            extra: Arc::new(std::mem::take(fields)),
        })
    }
    pub fn stamp(&self) -> Stamp {
        Stamp(self.identity.clone())
    }
    pub fn available(&self) -> bool {
        !matches!(self.data, Data::Unavailable(_))
    }
    pub fn items(&self) -> &[Stored] {
        match &self.data {
            Data::Ready(r) => &r.items,
            _ => &[],
        }
    }
    fn ready(&self) -> Result<Ready, Error> {
        match &self.data {
            Data::Empty => Ok(Ready {
                revision: 0,
                items: vec![],
                extra: Default::default(),
            }),
            Data::Ready(r) => Ok((**r).clone()),
            Data::Unavailable(_) => Err(Error::Unavailable),
        }
    }
    fn commit(&mut self, mut value: Ready) -> Result<(), Error> {
        value.revision = value
            .revision
            .checked_add(1)
            .filter(|v| *v <= MAX)
            .ok_or(Error::Limit)?;
        self.data = Data::Ready(Arc::new(value));
        self.identity = Arc::new(());
        Ok(())
    }
    pub fn visibility(&mut self, updates: &[(&str, bool)]) -> Result<(), Error> {
        let mut next = self.ready()?;
        for (id, visible) in updates {
            let item = next
                .items
                .iter_mut()
                .find(|i| i.attachment.id() == *id)
                .ok_or(Error::Conflict)?;
            item.visible = Some(*visible);
        }
        if updates.is_empty() {
            return Ok(());
        }
        self.commit(next)
    }
    pub fn upsert(
        &mut self,
        snapshot: &Snapshot,
        scope: &str,
        expected: u64,
        current: u64,
        mut value: Value,
    ) -> Result<(), Error> {
        if expected != current {
            return Err(Error::Stale);
        }
        let owner = snapshot.owner(scope).ok_or(Error::Owner)?;
        let fields = value.as_object_mut().ok_or(Error::Invalid)?;
        let prior = fields
            .remove("priorRevision")
            .and_then(|v| v.as_u64())
            .filter(|v| *v < MAX)
            .ok_or(Error::Invalid)?;
        fields.insert("pluginId".into(), owner.plugin.into());
        fields.insert("pluginVersion".into(), owner.version.into());
        fields.remove("overlayVisible");
        fields.remove("payloadDigest");
        let mut item = Stored::parse(value)?;
        let a = &item.attachment;
        if a.revision() != prior + 1 {
            return Err(Error::Conflict);
        }
        if !snapshot.declares(scope, "dataTypes", a.type_id())
            || a.renderer()
                .is_some_and(|id| !snapshot.declares(scope, "renderers", id))
        {
            return Err(Error::Owner);
        }
        item.digest = a
            .payload()
            .map(|p| Arc::from(crate::unit_attachment_json::digest(p)));
        let mut next = self.ready()?;
        let index = next.items.iter().position(|i| i.attachment.id() == a.id());
        if let Some(index) = index {
            let previous = &next.items[index].attachment;
            if previous.plugin() != owner.plugin || previous.revision() != prior {
                return Err(Error::Conflict);
            }
            next.items[index] = item;
        } else {
            if prior != 0 {
                return Err(Error::Conflict);
            }
            if next.items.len() >= 64 {
                return Err(Error::Limit);
            }
            next.items.push(item);
        }
        self.commit(next)
    }
    pub fn remove(
        &mut self,
        snapshot: &Snapshot,
        scope: &str,
        expected: u64,
        current: u64,
        id: &str,
        prior: u64,
    ) -> Result<(), Error> {
        if expected != current {
            return Err(Error::Stale);
        }
        let owner = snapshot.owner(scope).ok_or(Error::Owner)?;
        let mut next = self.ready()?;
        let index = next
            .items
            .iter()
            .position(|i| i.attachment.id() == id)
            .ok_or(Error::Conflict)?;
        let item = &next.items[index].attachment;
        if item.plugin() != owner.plugin || !id.starts_with(&format!("{}.", owner.plugin)) {
            return Err(Error::Owner);
        }
        if item.revision() != prior {
            return Err(Error::Conflict);
        }
        next.items.remove(index);
        self.commit(next)
    }
    /// 只在保存 worker 中物化 JSON；Empty 删除旧字段，Unavailable 保留其原始内容。
    pub fn write(&self, fields: &mut Map<String, Value>) -> Result<(), Error> {
        let value = match &self.data {
            Data::Empty => {
                fields.remove("extensionState");
                return Ok(());
            }
            Data::Unavailable(raw) => {
                serde_json::from_str(raw.get()).map_err(|_| Error::Invalid)?
            }
            Data::Ready(r) => {
                let mut fields = (*r.extra).clone();
                fields.insert("schemaVersion".into(), 1.into());
                fields.insert("revision".into(), r.revision.into());
                fields.insert(
                    "attachments".into(),
                    Value::Array(
                        r.items
                            .iter()
                            .map(Stored::value)
                            .collect::<Result<_, _>>()?,
                    ),
                );
                Value::Object(fields)
            }
        };
        fields.insert("extensionState".into(), value);
        Ok(())
    }
}
impl Serialize for Stored {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid extension attachment store: {self:?}")
    }
}
#[cfg(any(test, feature = "test-support"))]
#[path = "unit_attachment_store_tests.rs"]
pub mod tests;
