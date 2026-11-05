//! 同意图串行网络许可；state→registry 锁序，任何锁都不跨 HTTP，取消先撤销精确租约。
use super::*;
use crate::device_authorization_lease::Lease;
use std::{collections::BTreeMap, sync::Weak};
#[derive(Clone)]
pub struct Request {
    intent: Intent,
    identity_root: PathBuf,
}
impl Request {
    pub fn new(intent: Intent, identity_root: PathBuf) -> Self {
        Self {
            intent,
            identity_root,
        }
    }
    pub fn intent(&self) -> &Intent {
        &self.intent
    }
    pub(crate) fn same_intent(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.intent.store.0, &other.intent.store.0)
            && self.intent.record.same_intent(&other.intent.record)
    }
    /// 只读同 Store 的真实发布事实，不读磁盘、不授权；外部删文件不能伪造退休成功。
    pub(crate) fn published_retired(&self) -> bool {
        self.intent.store.0.state.lock().is_ok_and(|state| {
            !state
                .entries
                .iter()
                .any(|entry| entry.same_intent(&self.intent.record))
        })
    }
    pub(crate) fn cancel(&self) -> Result<(), Error> {
        self.intent.store.cancel(&self.intent)
    }
    pub(crate) fn activate(&self, lease: Arc<Lease>, cleanup: bool) -> Result<Active, Error> {
        let store = &self.intent.store;
        let state = store.0.state.lock().map_err(|_| Error::Changed)?;
        store.baseline(&state, lease.preparation_cancel())?;
        let record = state
            .entries
            .iter()
            .find(|old| old.same_intent(&self.intent.record))
            .ok_or(Error::Changed)?;
        if record.cancelled != cleanup || record.stop == Some(record::Stop::Unlinked) {
            return Err(Error::Cancelled);
        }
        lease
            .validate(record.prepared.envelope.origin(), now()?)
            .map_err(|_| Error::Stopped)?;
        if lease.device_id() != record.prepared.envelope.source_device() {
            return Err(Error::Identity);
        }
        let identity = crate::device_identity_store::Store::new(&self.identity_root)
            .map_err(|_| Error::Identity)?
            .load_existing()
            .map_err(|_| Error::Identity)?;
        record
            .prepared
            .envelope
            .verify_source(identity.identity())
            .map_err(|_| Error::Identity)?;
        let mut active = store.0.activity.0.lock().map_err(|_| Error::Changed)?;
        if active.contains_key(&record.selection) {
            return Err(Error::Busy);
        }
        active.insert(
            record.selection.clone(),
            Entry {
                envelope: record.prepared.envelope.clone(),
                lease: Arc::downgrade(&lease),
            },
        );
        Ok(Active {
            request: self.clone(),
            lease,
            cleanup,
        })
    }
}
struct Entry {
    envelope: Envelope,
    lease: Weak<Lease>,
}
#[derive(Default)]
pub(super) struct Registry(Mutex<BTreeMap<Selection, Entry>>);
impl Registry {
    pub(super) fn stop(&self, record: &Record) {
        let lease = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&record.selection)
            .filter(|entry| entry.envelope == record.prepared.envelope)
            .and_then(|entry| entry.lease.upgrade());
        if let Some(lease) = lease {
            lease.revoke();
        }
    }
}
pub(crate) struct Active {
    request: Request,
    lease: Arc<Lease>,
    cleanup: bool,
}
impl Active {
    pub(crate) fn request(&self) -> &Request {
        &self.request
    }
    pub(crate) fn intent(&self) -> &Intent {
        self.request.intent()
    }
    pub(crate) fn lease(&self) -> &Arc<Lease> {
        &self.lease
    }
    pub(crate) fn check(&self) -> Result<(), Error> {
        let intent = self.intent();
        self.lease
            .validate(intent.envelope().origin(), now()?)
            .map_err(|_| Error::Stopped)?;
        let state = intent.store.0.state.lock().map_err(|_| Error::Changed)?;
        intent
            .store
            .baseline(&state, self.lease.preparation_cancel())?;
        if !state.entries.iter().any(|old| {
            old.same_intent(&intent.record)
                && old.cancelled == self.cleanup
                && old.stop != Some(record::Stop::Unlinked)
        }) {
            return Err(Error::Changed);
        }
        Ok(())
    }
    pub(crate) fn read_body(&self) -> Result<Vec<u8>, Error> {
        self.check()?;
        if self.cleanup {
            return Err(Error::Cancelled);
        }
        serde_json::to_vec(&serde_json::json!({"projectionId":self.intent().envelope().projection_id(),"knownRevision":0})).map_err(|_| Error::Format)
    }
    pub(crate) fn create_body(&self) -> Result<Pixels, Error> {
        self.check()?;
        if self.cleanup {
            return Err(Error::Cancelled);
        }
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Body<'a> {
            envelope: &'a Envelope,
            snapshot: &'a snapshot::Snapshot,
            target_device_id: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            peer_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            remote_device_id: Option<&'a str>,
        }
        let intent = self.intent();
        let body = Body {
            envelope: intent.envelope(),
            snapshot: &intent.record.prepared.snapshot,
            target_device_id: intent.target(),
            peer_id: intent
                .record
                .selection
                .offline
                .as_ref()
                .map(|r| r.peer_id.as_str()),
            remote_device_id: intent
                .record
                .selection
                .offline
                .as_ref()
                .map(|r| r.remote_device_id.as_str()),
        };
        let bytes = crate::session_write::encode_bounded(&body, self.lease.preparation_cancel())
            .map_err(|_| Error::Limit)?;
        if bytes.len() > super::super::accept::MAX_RESPONSE {
            return Err(Error::Limit);
        }
        self.check()?;
        Ok(bytes)
    }
    /// 仅串行 HTTP owner 确认 unlinked:true 后调用；失败保留原取消记录。
    pub(crate) fn retire(&self) -> Result<(), Error> {
        self.check()?;
        if !self.cleanup {
            return Err(Error::Cancelled);
        }
        let intent = self.intent();
        let mut state = intent.store.0.state.lock().map_err(|_| Error::Changed)?;
        if let Some(index) = state.entries.iter().position(|old| {
            old.same_intent(&intent.record) && old.stop == Some(record::Stop::Requested)
        }) {
            let mut next = state.entries.clone();
            next[index].stop = Some(record::Stop::Unlinked);
            return intent
                .store
                .persist(&mut state, next, self.lease.preparation_cancel());
        }
        let next = state
            .entries
            .iter()
            .filter(|old| !old.same_intent(&intent.record))
            .cloned()
            .collect();
        intent
            .store
            .persist(&mut state, next, self.lease.preparation_cancel())
    }
}
impl Drop for Active {
    fn drop(&mut self) {
        let intent = self.intent();
        let mut active = intent
            .store
            .0
            .activity
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active
            .get(&intent.record.selection)
            .is_some_and(|entry| entry.lease.ptr_eq(&Arc::downgrade(&self.lease)))
        {
            active.remove(&intent.record.selection);
        }
    }
}
fn now() -> Result<u64, Error> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|time| u64::try_from(time.as_millis()).ok())
        .ok_or(Error::Stopped)
}
#[cfg(test)]
#[path = "projection_sender_activity_tests.rs"]
mod tests;
