//! 发送写前记录独立于 receiver；scope 是宿主稳定工作区身份的摘要，不存路径/凭据。
use super::snapshot::Snapshot;
use super::{check, Error, Selection};
use crate::projection_delivery::Envelope;
use serde::Serialize;
#[path = "projection_sender_wire.rs"]
mod wire;
use std::{
    collections::BTreeSet,
    sync::{atomic::AtomicBool, Arc},
};

pub const MAX_ENTRIES: usize = 64;
pub const MAX_BYTES: usize = 48 * 1024 * 1024;
pub(super) struct Prepared {
    pub(super) envelope: Envelope,
    pub(super) snapshot: Snapshot,
}
#[derive(Clone)]
pub(super) struct Record {
    pub(super) selection: Selection,
    pub(super) prepared: Arc<Prepared>,
    pub(super) cancelled: bool,
    pub(super) stop: Option<Stop>,
}
/// 仅显式稳定解绑使用；确认事实保留到 canonical metadata 的精确 ack。
#[derive(Clone, Copy, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) enum Stop {
    Requested,
    Unlinked,
}
impl Serialize for Record {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Fields<'a> {
            selection: &'a Selection,
            envelope: &'a Envelope,
            snapshot: &'a Snapshot,
            cancelled: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            stop: Option<Stop>,
        }
        Fields {
            selection: &self.selection,
            envelope: &self.prepared.envelope,
            snapshot: &self.prepared.snapshot,
            cancelled: self.cancelled,
            stop: self.stop,
        }
        .serialize(serializer)
    }
}
#[derive(Serialize)]
pub(super) struct Document<'a> {
    version: u32,
    entries: &'a [Record],
}
impl Record {
    pub(super) fn same_intent(&self, other: &Self) -> bool {
        self.selection == other.selection && self.prepared.envelope == other.prepared.envelope
    }
}
impl<'a> Document<'a> {
    pub(super) fn new(entries: &'a [Record]) -> Self {
        Self {
            version: 1,
            entries,
        }
    }
    pub(super) fn decode(bytes: &[u8], cancel: &AtomicBool) -> Result<Vec<Record>, Error> {
        check(cancel)?;
        if bytes.len() > MAX_BYTES {
            return Err(Error::Limit);
        }
        let wire = wire::decode(bytes)?;
        let mut keys = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut entries = Vec::with_capacity(wire.len());
        for value in wire {
            check(cancel)?;
            let selection = value.selection()?;
            let envelope = value.envelope()?;
            envelope
                .validate(&selection.origin)
                .map_err(|_| Error::Format)?;
            if (value.stop.is_some() && !value.cancelled)
                || envelope.source_unit() != selection.unit
                || envelope.source_revision() != 1
                || !keys.insert(selection.clone())
                || !ids.insert((
                    envelope.origin().to_owned(),
                    envelope.projection_id().to_owned(),
                ))
            {
                return Err(Error::Format);
            }
            let snapshot = Snapshot::decode(value.snapshot, cancel)?;
            if snapshot.digest != envelope.content_digest() {
                return Err(Error::Snapshot);
            }
            entries.push(Record {
                selection,
                prepared: Arc::new(Prepared { envelope, snapshot }),
                cancelled: value.cancelled,
                stop: value.stop,
            });
        }
        check(cancel)?;
        Ok(entries)
    }
}
